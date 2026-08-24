use super::*;

fn entry(access: &str, display: &str) -> Entry {
    Entry {
        access: PathBuf::from(access),
        display: PathBuf::from(display),
    }
}

/// A plain entry, where nothing stands between the application and the file
fn plain(path: &str) -> Entry {
    entry(path, path)
}

fn history(paths: &[&str]) -> Vec<Entry> {
    paths.iter().map(|p| plain(p)).collect()
}

#[test]
fn a_new_file_goes_to_the_front_and_the_rest_keeps_its_order() {
    let before = history(&["/a.collomatique", "/b.collomatique"]);

    assert_eq!(
        merge(before, plain("/c.collomatique")),
        vec![
            plain("/c.collomatique"),
            plain("/a.collomatique"),
            plain("/b.collomatique"),
        ]
    );
}

#[test]
fn reopening_a_known_file_refreshes_it_without_duplicating_it() {
    let before = history(&["/a.collomatique", "/b.collomatique", "/c.collomatique"]);

    assert_eq!(
        merge(before, plain("/c.collomatique")),
        vec![
            plain("/c.collomatique"),
            plain("/a.collomatique"),
            plain("/b.collomatique"),
        ]
    );
}

#[test]
fn the_displayed_path_is_what_tells_two_entries_apart() {
    // The same document handed over twice by the document portal, under a
    // different access path each time: one entry, the newer access path.
    let before = vec![entry(
        "/run/user/1000/doc/aaa/x.collomatique",
        "/home/me/x.collomatique",
    )];
    let again = entry(
        "/run/user/1000/doc/bbb/x.collomatique",
        "/home/me/x.collomatique",
    );

    assert_eq!(merge(before, again.clone()), vec![again]);
}

#[test]
fn a_different_file_at_the_same_access_path_is_a_different_entry() {
    let before = vec![entry(
        "/run/user/1000/doc/aaa/x.collomatique",
        "/home/me/x.collomatique",
    )];
    let other = entry(
        "/run/user/1000/doc/aaa/x.collomatique",
        "/media/key/x.collomatique",
    );

    assert_eq!(merge(before, other.clone()).len(), 2);
}

#[test]
fn the_history_stops_at_history_length() {
    let mut current = Vec::new();
    for i in 0..(HISTORY_LENGTH + 3) {
        current = merge(current, plain(&format!("/{i}.collomatique")));
    }

    // Eight files went in, five are left: the newest in front, and the three
    // oldest gone.
    assert_eq!(current.len(), HISTORY_LENGTH);
    assert_eq!(
        current.first(),
        Some(&plain(&format!("/{}.collomatique", HISTORY_LENGTH + 2)))
    );
    assert_eq!(current.last(), Some(&plain("/3.collomatique")));
}

#[test]
fn an_unreadable_body_remembers_nothing() {
    assert_eq!(parse(""), Vec::new());
    assert_eq!(parse("not json"), Vec::new());
    assert_eq!(parse("{\"entries\": []}"), Vec::new());
    // A list, but not of entries.
    assert_eq!(parse("[\"/a.collomatique\"]"), Vec::new());
}

#[test]
fn the_body_round_trips() {
    let current = vec![
        entry(
            "/run/user/1000/doc/aaa/x.collomatique",
            "/home/me/x.collomatique",
        ),
        plain("/home/me/y.collomatique"),
    ];

    assert_eq!(parse(&render(&current).unwrap()), current);
}

#[test]
fn the_body_is_a_bare_list_of_entries() {
    let current = vec![plain("/home/me/y.collomatique")];

    assert_eq!(
        render(&current).unwrap(),
        "[{\"access\":\"/home/me/y.collomatique\",\"display\":\"/home/me/y.collomatique\"}]"
    );
}
