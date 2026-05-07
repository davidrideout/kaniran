//! Port of the `ichiran/custom` package — supplementary data sources
//! (municipalities, wards, etc.) that augment the JMdict-derived core
//! with proper-noun coverage. Initial scope (2026-05-07): in-memory
//! row records for the two non-XML loaders. The CSV loader classes
//! (`municipality-csv`, `ward-csv`) and the XML-driven `xml-entry`
//! loader land later — XML ingest is out of scope per the resolved
//! decision (see `reverse/scripts/HANDOFF.md`).

pub mod municipality_struct;
pub mod ward_struct;
