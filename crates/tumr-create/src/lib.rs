//! TUMR Create (CRT-A): the one project-creation engine behind `tumr create` (PowerShell) and the
//! desktop Create Project wizard. Both front ends reach it through the Runtime Core
//! (`project.create.*`), so they cannot behave differently.
//!
//! TUMR prepares the project and its environment; the coding agent builds the app. A created
//! project is a minimal, valid scaffold from the framework's official tooling (never a sample
//! application).
//!
//! | Module | Purpose |
//! |---|---|
//! | [`framework`] | The five choices (Expo, React Native, Flutter, Kotlin, Java) and what they map to |
//! | [`request`] | What the user chose, validated: name, package id, destination |
//! | [`metadata`] | `.tumr/project.json`, how TUMR recognises a project it created |
//! | [`creators`] | The five framework creators and the tools they run |
//! | [`job`] | The staged run: prepare → scaffold → metadata → verify, with rollback |
//!
//! [`creators`] (CRT-B) holds one creator per framework, each the framework's official tool (Java:
//! TUMR's template); the [`job::Creator`] interface is what the engine runs.

pub mod creators;
pub mod framework;
pub mod job;
pub mod metadata;
pub mod request;

pub use framework::Framework;
pub use job::{run, CreateError, Created, Creator, Progress, Stage};
pub use metadata::ProjectMetadata;
pub use request::{CreateRequest, Destination, Validated};
