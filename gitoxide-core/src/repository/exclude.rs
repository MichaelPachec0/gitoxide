use std::borrow::Cow;
use std::io;

use anyhow::{anyhow, bail};
use gix::bstr::BStr;

use crate::{repository::PathsOrPatterns, OutputFormat};

pub mod query {
    use std::ffi::OsString;

    use crate::OutputFormat;

    pub struct Options {
        pub format: OutputFormat,
        pub overrides: Vec<OsString>,
        pub show_ignore_patterns: bool,
        pub statistics: bool,
    }
}

pub fn query(
    repo: gix::Repository,
    input: PathsOrPatterns,
    mut out: impl io::Write,
    mut err: impl io::Write,
    query::Options {
        overrides,
        format,
        show_ignore_patterns,
        statistics,
    }: query::Options,
) -> anyhow::Result<()> {
    if format != OutputFormat::Human {
        bail!("JSON output isn't implemented yet");
    }

    let index = repo.index()?;
    let mut cache = repo.excludes(
        &index,
        Some(gix::ignore::Search::from_overrides(&mut overrides.into_iter())),
        Default::default(),
    )?;

    match input {
        PathsOrPatterns::Paths(paths) => {
            for path in paths {
                let is_dir = gix::path::from_bstr(Cow::Borrowed(path.as_ref()))
                    .metadata()
                    .ok()
                    .map(|m| m.is_dir());
                let entry = cache.at_entry(path.as_slice(), is_dir)?;
                let match_ = entry
                    .matching_exclude_pattern()
                    .and_then(|m| (show_ignore_patterns || !m.pattern.is_negative()).then_some(m));
                print_match(match_, path.as_ref(), &mut out)?;
            }
        }
        PathsOrPatterns::Patterns(patterns) => {
            for (path, _entry) in repo
                .pathspec(
                    patterns.into_iter(),
                    repo.work_dir().is_some(),
                    &index,
                    gix::worktree::stack::state::attributes::Source::WorktreeThenIdMapping
                        .adjust_for_bare(repo.is_bare()),
                )?
                .index_entries_with_paths(&index)
                .ok_or_else(|| anyhow!("Pathspec didn't yield any entry"))?
            {
                let entry = cache.at_entry(path, Some(false))?;
                let match_ = entry
                    .matching_exclude_pattern()
                    .and_then(|m| (show_ignore_patterns || !m.pattern.is_negative()).then_some(m));
                print_match(match_, path, &mut out)?;
            }
        }
    }

    if let Some(stats) = statistics.then(|| cache.take_statistics()) {
        out.flush()?;
        writeln!(err, "{stats:#?}").ok();
    }
    Ok(())
}

fn print_match(
    m: Option<gix::ignore::search::Match<'_>>,
    path: &BStr,
    mut out: impl std::io::Write,
) -> std::io::Result<()> {
    match m {
        Some(m) => writeln!(
            out,
            "{}:{}:{}\t{}",
            m.source.map(std::path::Path::to_string_lossy).unwrap_or_default(),
            m.sequence_number,
            m.pattern,
            path
        ),
        None => writeln!(out, "::\t{path}"),
    }
}
