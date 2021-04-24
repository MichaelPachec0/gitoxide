mod entries {
    mod simple_compression {
        use crate::fixture_path;
        use git_features::progress;
        use git_odb::{compound, linked, pack, pack::data::encode};
        use std::{path::PathBuf, sync::Arc};

        enum DbKind {
            AbunchOfRandomObjects,
        }

        fn db(kind: DbKind) -> crate::Result<Arc<linked::Db>> {
            use DbKind::*;
            let path: PathBuf = match kind {
                AbunchOfRandomObjects => fixture_path("objects"),
            };
            linked::Db::at(path).map_err(Into::into).map(Into::into)
        }

        #[test]
        #[should_panic]
        fn all_input_objects() {
            (|| -> crate::Result {
                let db = db(DbKind::AbunchOfRandomObjects)?;
                let obj_count = db.iter().count();
                assert_eq!(obj_count, 146);
                let all_objects = db.arc_iter().flat_map(Result::ok);
                let entries: Vec<_> = encode::entries(
                    db.clone(),
                    || pack::cache::Noop,
                    all_objects,
                    progress::Discard,
                    encode::entries::Options::default(),
                )
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .flatten()
                .collect();
                assert_eq!(entries.len(), obj_count, "each object gets one entry");

                let mut pack_file = tempfile::tempfile()?;
                let num_written_bytes = {
                    let num_entries = entries.len();
                    let mut pack_writer = encode::write::Entries::new(
                        std::iter::once(Ok::<_, encode::entries::Error<compound::locate::Error>>(entries)),
                        &mut pack_file,
                        num_entries as u32,
                        pack::data::Version::V2,
                        git_hash::Kind::Sha1,
                    );
                    let n = pack_writer.next().expect("one entries bundle was written")?;
                    assert!(
                        pack_writer.next().is_none(),
                        "there is nothing more to iterate this time"
                    );
                    // verify we can still get the original parts back
                    let _ = pack_writer.input;
                    let _ = pack_writer.into_write();
                    n
                };
                assert_eq!(
                    num_written_bytes,
                    pack_file.metadata()?.len(),
                    "it reports the correct amount of written bytes"
                );

                Ok(())
            })()
            .unwrap();
        }
    }
}
