"""
ichiran_worker_pool.py — Async pool of persistent SBCL ichiran workers.

Each worker is a long-lived subprocess running persistent_worker.lisp.
Queries are sent as JSON lines on stdin; responses come back as JSON lines on stdout.
"""

import asyncio
import json
import logging
import os
import subprocess
import time
from dataclasses import dataclass, field

logger = logging.getLogger("ichiran_worker_pool")

# Path to the Lisp worker script and SBCL core (override via env)
DEFAULT_WORKER_SCRIPT = "./full_trace_worker.lisp"
DEFAULT_SBCL_CORE = "./ichiran.core"
DEFAULT_POOL_SIZE = 4
WORKER_STARTUP_TIMEOUT = 30.0  # seconds to wait for worker to respond to ping
QUERY_TIMEOUT = 30.0  # seconds per query


@dataclass
class Worker:
    """A single persistent SBCL subprocess."""

    id: int
    process: asyncio.subprocess.Process | None = None
    lock: asyncio.Lock = field(default_factory=asyncio.Lock)
    healthy: bool = False
    _start_time: float = 0.0

    async def start(
        self,
        sbcl_core: str = DEFAULT_SBCL_CORE,
        worker_script: str = DEFAULT_WORKER_SCRIPT,
    ):
        """Launch the SBCL subprocess."""
        cmd = [
            "sbcl",
            "--core", sbcl_core,
            "--noinform",
            "--non-interactive",
            "--load", worker_script,
        ]
        logger.info("Worker %d starting: %s", self.id, " ".join(cmd))
        self._start_time = time.monotonic()
        self.process = await asyncio.create_subprocess_exec(
            # 100MB line limit for trace output. The JSON projector inflates
            # per-sentence response size (each capture serialises args/result
            # as a JSON tree with `_meta` envelopes) and the response includes
            # *every* capture from every installed FQN — dedup happens later
            # in FqnWriter, not on the worker — so a single complex sentence
            # can emit tens of MB. 10MB was chosen for the s-expr encoder and
            # is too tight for JSON.
            limit=100 * 1024 * 1024,
            *cmd,
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        # Verify the worker is alive with a ping
        try:
            resp = await self._send_raw('{"op":"ping"}')
            if resp and resp.get("ok"):
                self.healthy = True
                elapsed = time.monotonic() - self._start_time
                logger.info("Worker %d ready (%.1fs startup)", self.id, elapsed)
            else:
                logger.error("Worker %d ping failed: %s", self.id, resp)
                self.healthy = False
        except Exception as e:
            logger.error("Worker %d startup failed: %s", self.id, e)
            self.healthy = False

    async def _send_raw(self, line: str, timeout: float = WORKER_STARTUP_TIMEOUT) -> dict | None:
        """Send a raw JSON line and read the response."""
        if self.process is None or self.process.returncode is not None:
            return None

        try:
            self.process.stdin.write((line + "\n").encode("utf-8"))
            await self.process.stdin.drain()
            raw = await asyncio.wait_for(
                self.process.stdout.readline(), timeout=timeout
            )
            if not raw:
                return None
            return json.loads(raw.decode("utf-8"))
        except asyncio.TimeoutError:
            logger.error("Worker %d timed out", self.id)
            return None
        except Exception as e:
            logger.error("Worker %d send error: %s", self.id, e)
            return None

    async def query(self, op: str, **kwargs) -> dict:
        """Send a query and return the parsed response.

        Raises RuntimeError if the worker is dead or returns invalid data.
        """
        payload = json.dumps({"op": op, **kwargs}, ensure_ascii=False)
        resp = await self._send_raw(payload, timeout=QUERY_TIMEOUT)
        if resp is None:
            self.healthy = False
            raise RuntimeError(f"Worker {self.id} did not respond")
        if not resp.get("ok"):
            raise RuntimeError(resp.get("error", "unknown error"))
        return resp["result"]

    async def stop(self):
        """Gracefully stop the worker."""
        if self.process and self.process.returncode is None:
            try:
                self.process.stdin.write(b'{"op":"quit"}\n')
                await self.process.stdin.drain()
                await asyncio.wait_for(self.process.wait(), timeout=5.0)
            except Exception:
                self.process.kill()
                await self.process.wait()
        self.healthy = False
        self.process = None

    def is_alive(self) -> bool:
        return (
            self.process is not None
            and self.process.returncode is None
            and self.healthy
        )


class WorkerPool:
    """Pool of N persistent SBCL workers with async acquire/release."""

    def __init__(
        self,
        size: int | None = None,
        sbcl_core: str = DEFAULT_SBCL_CORE,
        worker_script: str = DEFAULT_WORKER_SCRIPT,
    ):
        self.size = size or int(os.environ.get("ICHIRAN_POOL_SIZE", DEFAULT_POOL_SIZE))
        self.sbcl_core = sbcl_core
        self.worker_script = worker_script
        self._workers: list[Worker] = []
        self._semaphore: asyncio.Semaphore | None = None
        self._available: asyncio.Queue | None = None
        self._started = False

    async def start(self):
        """Start all workers. Call once at application startup."""
        if self._started:
            return
        logger.info("Starting worker pool (size=%d)", self.size)
        self._semaphore = asyncio.Semaphore(self.size)
        self._available = asyncio.Queue(maxsize=self.size)

        # Start workers in parallel
        workers = [Worker(id=i) for i in range(self.size)]
        await asyncio.gather(
            *(w.start(self.sbcl_core, self.worker_script) for w in workers)
        )

        for w in workers:
            self._workers.append(w)
            if w.is_alive():
                await self._available.put(w)
            else:
                logger.warning("Worker %d failed to start, will retry on acquire", w.id)

        self._started = True
        healthy = sum(1 for w in self._workers if w.is_alive())
        logger.info("Worker pool started: %d/%d healthy", healthy, self.size)

    async def stop(self):
        """Stop all workers. Call at application shutdown."""
        logger.info("Stopping worker pool")
        await asyncio.gather(*(w.stop() for w in self._workers))
        self._workers.clear()
        self._started = False

    async def _ensure_healthy(self, worker: Worker) -> Worker:
        """Restart a worker if it is not healthy."""
        if not worker.is_alive():
            logger.warning("Worker %d is dead, restarting", worker.id)
            await worker.stop()
            await worker.start(self.sbcl_core, self.worker_script)
            if not worker.is_alive():
                raise RuntimeError(f"Worker {worker.id} failed to restart")
        return worker

    async def execute(self, op: str, **kwargs) -> any:
        """Acquire a worker, send a query, return the result.

        This is the main entry point for callers.
        """
        worker = await self._available.get()
        try:
            worker = await self._ensure_healthy(worker)
            return await worker.query(op, **kwargs)
        finally:
            await self._available.put(worker)

    async def broadcast(self, op: str, **kwargs) -> list:
        """Run OP on every worker in parallel. Used for setup/teardown
        ops that mutate worker-local state (install hooks, clear capture
        buffer) — every worker has its own SBCL process and its own
        :ichi-trace state, so install-once-on-the-pool means
        install-on-each.

        Drains the available queue first so no other request grabs a
        worker mid-broadcast; refills it on completion. Returned list
        is one entry per worker, in queue-acquisition order; entries
        may be exception instances if a worker failed.
        """
        held = []
        for _ in range(self.size):
            held.append(await self._available.get())
        try:
            healthy = await asyncio.gather(
                *(self._ensure_healthy(w) for w in held),
                return_exceptions=True,
            )
            results = await asyncio.gather(
                *(
                    w.query(op, **kwargs)
                    for w in healthy
                    if isinstance(w, Worker)
                ),
                return_exceptions=True,
            )
            return list(results)
        finally:
            for w in held:
                await self._available.put(w)

    async def health_check(self) -> dict:
        """Return pool health status."""
        return {
            "pool_size": self.size,
            "healthy": sum(1 for w in self._workers if w.is_alive()),
            "workers": [
                {"id": w.id, "alive": w.is_alive(), "pid": w.process.pid if w.process else None}
                for w in self._workers
            ],
        }
