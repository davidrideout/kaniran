"""
ichiran_main_pooled.py — FastAPI service over a persistent SBCL extractor pool.

Each worker is a long-lived SBCL process with ichiran loaded plus the
:ichi-trace package (see extractor_worker.lisp + trace_capture.lisp).
Routes:

    GET  /ping             liveness, no worker hit
    GET  /health           pool census
    GET  /installed        list of FQNs currently encapsulated on worker 0
    POST /install          install hooks on every worker
                           body: {"fqns": [SPEC, ...]}
                             SPEC = "FQN" | {"fqn": "FQN",
                                             "result_projector": "NAME"}
                           projector NAME resolves via :ichi-projectors
                           in projectors.lisp on the worker.
    POST /clear            drop all buffered captures on every worker
    POST /uninstall-all    remove all hooks on every worker
    POST /extract          run entry-points on text, return drained captures
                           body: {"text": "今日は"}
                           response: {"captures":[{fn,args,result},...], "skipped":N}

Environment:
    ICHIRAN_POOL_SIZE      number of SBCL workers (default: 4)
    ICHIRAN_SBCL_CORE      path to SBCL core (default: /root/ichiran.core)
    ICHIRAN_WORKER_SCRIPT  path to extractor_worker.lisp (default: ./extractor_worker.lisp)

Usage:
    uvicorn ichiran_main_pooled:app --host 0.0.0.0 --port 9100
"""

import logging
import os
from contextlib import asynccontextmanager
from datetime import datetime

from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

from ichiran_worker_pool import WorkerPool

logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s %(name)s %(levelname)s %(message)s",
)
logger = logging.getLogger("ichiran_extractor")

POOL_SIZE = int(os.environ.get("ICHIRAN_POOL_SIZE", "4"))
SBCL_CORE = os.environ.get("ICHIRAN_SBCL_CORE", "/root/ichiran.core")
WORKER_SCRIPT = os.environ.get("ICHIRAN_WORKER_SCRIPT", "./extractor_worker.lisp")

pool = WorkerPool(size=POOL_SIZE, sbcl_core=SBCL_CORE, worker_script=WORKER_SCRIPT)


@asynccontextmanager
async def lifespan(app: FastAPI):
    await pool.start()
    yield
    await pool.stop()


app = FastAPI(title="Ichiran Extractor (pooled)", lifespan=lifespan)


class InstallSpec(BaseModel):
    fqn: str
    result_projector: str | None = None


class InstallBody(BaseModel):
    # Each element is either a bare FQN string or an InstallSpec dict
    # (the latter selects a projector by name from :ichi-projectors).
    fqns: list[str | InstallSpec]


class ExtractBody(BaseModel):
    text: str


@app.get("/ping")
async def ping():
    return {"time": datetime.now().isoformat()}


@app.get("/health")
async def health():
    return await pool.health_check()


@app.get("/installed")
async def installed():
    """Return the install set from worker 0. After /install all workers
    carry the same set, so one read is representative."""
    try:
        return await pool.execute("installed")
    except RuntimeError as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.post("/install")
async def install(body: InstallBody):
    """Install encapsulate hooks for every FQN on every worker."""
    if not body.fqns:
        raise HTTPException(status_code=400, detail="fqns must be non-empty")
    # Convert InstallSpec models back to plain dicts so the JSON the
    # pool sends to SBCL stays {"fqn":..., "result_projector":...}
    # rather than InstallSpec(...) repr.
    fqns_payload = [
        item if isinstance(item, str) else item.model_dump()
        for item in body.fqns
    ]
    try:
        results = await pool.broadcast("install", fqns=fqns_payload)
        # Each result is the worker's installed-count after the call.
        return {"workers": len(results), "installed_per_worker": results}
    except RuntimeError as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.post("/clear")
async def clear():
    """Drop all buffered captures on every worker."""
    try:
        await pool.broadcast("clear")
        return {"ok": True}
    except RuntimeError as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.post("/uninstall-all")
async def uninstall_all():
    """Remove every encapsulate hook on every worker."""
    try:
        await pool.broadcast("uninstall-all")
        return {"ok": True}
    except RuntimeError as e:
        raise HTTPException(status_code=500, detail=str(e))


@app.post("/extract")
async def extract(body: ExtractBody):
    """Drive the worker's entry-point sweep over `text` and return all
    captures since the last extract/clear. Routes to one worker; the
    worker's local capture buffer is drained on the way out."""
    if not body.text or not body.text.strip():
        raise HTTPException(status_code=400, detail="text must be non-empty")
    try:
        return await pool.execute("extract", text=body.text)
    except RuntimeError as e:
        logger.error("extract failed for %r: %s", body.text, e)
        raise HTTPException(status_code=500, detail=str(e))
