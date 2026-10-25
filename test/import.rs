mod support;

use gym::import;
use std::fs;
use support::Scratch;

fn write(p: &std::path::Path, s: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, s).unwrap();
}

/// The smallest archive the importer accepts: two places, two exercises, two
/// muscle groups, one food, one session with two sets, one meal, one weigh-in.
fn archive(s: &Scratch) -> std::path::PathBuf {
    let d = s.path().join("archive");
    write(
        &d.join("muscle_group.toml"),
        "[e.0]\nname = \"pecs\"\ndesc = \"\"\n",
    );
    write(
        &d.join("place.toml"),
        "[e.0]\nname = \"home\"\ndesc = \"\"\n\n[e.1]\nname = \"gym\"\ndesc = \"\"\n",
    );
    write(
        &d.join("exercise.toml"),
        "[e.0]\nname = \"bench press\"\ndesc = \"\"\n\n[[e.0.muscle_groups]]\ngroup = 0\namount = 1.0\n\n[e.1]\nname = \"side delts\"\ndesc = \"\"\nmuscle_groups = []\n",
    );
    write(
        &d.join("food.toml"),
        "[e.0]\nname = \"egg\"\ncalories = 80.0\nprotein = 6.0\nfat = 5.0\ncarbohydrate = 0.1\ndesc = \"\"\n",
    );
    write(
        &d.join("session/2026-01.toml"),
        "[e.3]\ndate = \"2026-01-02T10:00:00+00:00\"\nplace = 1\nsets = [7, 8]\ndesc = \"push\"\n",
    );
    write(
        &d.join("set/2026-01-02.toml"),
        "[e.7]\ndate = \"2026-01-02T10:01:00+00:00\"\nsession = 0\nexercise = 0\nload = 60.0\nrep = 8.0\ndesc = \"Fail.\"\n\n[e.8]\ndate = \"2026-01-02T10:02:00+00:00\"\nsession = 0\nexercise = 0\nload = 60.0\nrep = 6.0\ndesc = \"\"\n",
    );
    write(
        &d.join("meal/2026-01-02.toml"),
        "[e.5]\ndate = \"2026-01-02T12:00:00+00:00\"\nfood = 0\namount = 2.0\ndesc = \"lunch\"\n",
    );
    write(
        &d.join("weight/2026-01.toml"),
        "[e.2]\ndate = \"2026-01-02T07:00:00+00:00\"\nkg = 70.0\nbodyfat = 14.0\ndesc = \"\"\n",
    );
    d
}

#[tokio::test]
async fn the_archive_arrives_with_its_ids() {
    let s = Scratch::new("import");
    let d = archive(&s);
    let db = gym::db::Db::create(&s.db()).await.unwrap();
    let r = import::import(&db, &d, None).await.unwrap();
    assert_eq!(r.muscle_groups, 1);
    assert_eq!(r.places, 2);
    assert_eq!(r.exercises, 2);
    assert_eq!(r.exercise_muscle_groups, 1);
    assert_eq!(r.foods, 1);
    assert_eq!(r.sessions, 1);
    assert_eq!(r.sets, 2);
    assert_eq!(r.weights, 1);
    assert_eq!(r.meals, 1);

    let ids = sqlx::query_scalar!(r#"SELECT id AS "id!" FROM training_set ORDER BY position"#)
        .fetch_all(&db.pool)
        .await
        .unwrap();
    assert_eq!(ids, vec![7, 8], "old set ids survive");
    let sid = sqlx::query_scalar!(r#"SELECT id AS "id!" FROM session"#)
        .fetch_one(&db.pool)
        .await
        .unwrap();
    assert_eq!(sid, 3);

    let c = import::counts(&db).await.unwrap();
    assert_eq!(c.training_set, 2);
    assert_eq!(c.total(), 2 + 2 + 2 + 1 + 1 + 1 + 1 + 1);
}

#[tokio::test]
async fn the_session_list_decides_membership_not_the_sets_field() {
    let s = Scratch::new("membership");
    let d = archive(&s);
    write(
        &d.join("set/2026-01-02.toml"),
        "[e.7]\ndate = \"2026-01-02T10:01:00+00:00\"\nsession = 99\nexercise = 0\nload = 60.0\nrep = 8.0\ndesc = \"\"\n\n[e.8]\ndate = \"2026-01-02T10:02:00+00:00\"\nsession = 99\nexercise = 0\nload = 60.0\nrep = 6.0\ndesc = \"\"\n",
    );
    let db = gym::db::Db::create(&s.db()).await.unwrap();
    import::import(&db, &d, None).await.unwrap();
    let sessions = sqlx::query_scalar!(r#"SELECT DISTINCT session AS "s!" FROM training_set"#)
        .fetch_all(&db.pool)
        .await
        .unwrap();
    assert_eq!(sessions, vec![3], "the bogus session field is ignored");
}

#[tokio::test]
async fn a_set_no_session_lists_stops_the_import() {
    let s = Scratch::new("orphan");
    let d = archive(&s);
    write(
        &d.join("set/2026-01-03.toml"),
        "[e.9]\ndate = \"2026-01-03T10:00:00+00:00\"\nsession = 0\nexercise = 0\nload = 60.0\nrep = 8.0\ndesc = \"\"\n",
    );
    let db = gym::db::Db::create(&s.db()).await.unwrap();
    let e = import::import(&db, &d, None).await;
    let msg = format!("{}", e.err().expect("an orphan must be refused"));
    assert!(msg.contains("not listed by any session"), "{msg}");
    assert!(msg.contains('9'), "{msg}");
    let c = import::counts(&db).await.unwrap();
    assert_eq!(c.total(), 0, "a failed import leaves nothing behind");
}

#[tokio::test]
async fn a_set_listed_by_two_sessions_stops_the_import() {
    let s = Scratch::new("shared");
    let d = archive(&s);
    write(
        &d.join("session/2026-02.toml"),
        "[e.4]\ndate = \"2026-02-02T10:00:00+00:00\"\nplace = 1\nsets = [8]\ndesc = \"\"\n",
    );
    let db = gym::db::Db::create(&s.db()).await.unwrap();
    let e = import::import(&db, &d, None).await;
    let msg = format!("{}", e.err().expect("a shared set must be refused"));
    assert!(msg.contains("listed by session"), "{msg}");
}

#[tokio::test]
async fn a_repeated_id_inside_one_list_is_collapsed() {
    let s = Scratch::new("repeat");
    let d = archive(&s);
    write(
        &d.join("session/2026-01.toml"),
        "[e.3]\ndate = \"2026-01-02T10:00:00+00:00\"\nplace = 1\nsets = [7, 8, 8, 7]\ndesc = \"push\"\n",
    );
    let db = gym::db::Db::create(&s.db()).await.unwrap();
    let r = import::import(&db, &d, None).await.unwrap();
    assert_eq!(r.sets, 2);
    assert_eq!(r.duplicate_refs.len(), 2, "{:?}", r.duplicate_refs);
}

#[tokio::test]
async fn a_missing_reference_stops_the_import() {
    let s = Scratch::new("dangling");
    let d = archive(&s);
    write(
        &d.join("meal/2026-01-02.toml"),
        "[e.5]\ndate = \"2026-01-02T12:00:00+00:00\"\nfood = 9\namount = 1.0\ndesc = \"\"\n",
    );
    let db = gym::db::Db::create(&s.db()).await.unwrap();
    let e = import::import(&db, &d, None).await;
    assert!(format!("{}", e.err().unwrap()).contains("food"));
}

#[tokio::test]
async fn a_superset_note_becomes_a_group_when_the_mapping_resolves_it() {
    let s = Scratch::new("superset");
    let d = archive(&s);
    write(
        &d.join("set/2026-01-02.toml"),
        "[e.7]\ndate = \"2026-01-02T10:01:00+00:00\"\nsession = 0\nexercise = 0\nload = 60.0\nrep = 8.0\ndesc = \"Super set side delts 18 reps\"\n\n[e.8]\ndate = \"2026-01-02T10:02:00+00:00\"\nsession = 0\nexercise = 0\nload = 60.0\nrep = 6.0\ndesc = \"\"\n",
    );
    let map = s.path().join("map.toml");
    write(&map, "[map]\n\"side delts\" = \"side delts\"\n");

    let db = gym::db::Db::create(&s.db()).await.unwrap();
    let r = import::import(&db, &d, Some(&map)).await.unwrap();
    assert_eq!(r.supersets, 1);
    assert_eq!(r.inferred, 1);
    assert_eq!(r.sets, 3, "two logged sets plus the inferred partner");

    let rows = sqlx::query!(
        r#"SELECT exercise AS "exercise!", load AS "load!", rep AS "rep!", superset, inferred AS "inferred!" FROM training_set ORDER BY position"#
    )
    .fetch_all(&db.pool)
    .await
    .unwrap();
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[1].exercise, 1, "the partner exercise");
    assert_eq!(rows[1].rep, 18.0, "the reps from the note");
    assert_eq!(rows[1].load, rows[0].load, "a superset shares the load");
    assert_eq!(rows[1].superset, rows[0].superset);
    assert!(rows[1].superset.is_some());
    assert_eq!(rows[1].inferred, 1);
    assert_eq!(rows[2].superset, None, "the second set is standalone");
}

#[tokio::test]
async fn an_unmapped_partner_is_reported_and_left_alone() {
    let s = Scratch::new("unmapped");
    let d = archive(&s);
    write(
        &d.join("set/2026-01-02.toml"),
        "[e.7]\ndate = \"2026-01-02T10:01:00+00:00\"\nsession = 0\nexercise = 0\nload = 60.0\nrep = 8.0\ndesc = \"Super set sode delts 15 reps\"\n\n[e.8]\ndate = \"2026-01-02T10:02:00+00:00\"\nsession = 0\nexercise = 0\nload = 60.0\nrep = 6.0\ndesc = \"\"\n",
    );
    let map = s.path().join("map.toml");
    write(&map, "[map]\n\"side delts\" = \"side delts\"\n");
    let db = gym::db::Db::create(&s.db()).await.unwrap();
    let r = import::import(&db, &d, Some(&map)).await.unwrap();
    assert_eq!(r.supersets, 0);
    assert_eq!(r.superset_unmapped.len(), 1, "{:?}", r.superset_unmapped);
    assert!(r.superset_unmapped[0].contains("sode delts"));
}

#[tokio::test]
async fn a_partner_that_is_the_same_exercise_is_reported() {
    let s = Scratch::new("self");
    let d = archive(&s);
    write(
        &d.join("set/2026-01-02.toml"),
        "[e.7]\ndate = \"2026-01-02T10:01:00+00:00\"\nsession = 0\nexercise = 1\nload = 20.0\nrep = 8.0\ndesc = \"Super set side delts 18 reps\"\n\n[e.8]\ndate = \"2026-01-02T10:02:00+00:00\"\nsession = 0\nexercise = 1\nload = 20.0\nrep = 6.0\ndesc = \"\"\n",
    );
    let map = s.path().join("map.toml");
    write(&map, "[map]\n\"side delts\" = \"side delts\"\n");
    let db = gym::db::Db::create(&s.db()).await.unwrap();
    let r = import::import(&db, &d, Some(&map)).await.unwrap();
    assert_eq!(r.supersets, 0);
    assert_eq!(r.superset_self.len(), 1);
}

#[tokio::test]
async fn the_report_lists_what_it_could_not_infer() {
    let s = Scratch::new("report");
    let d = archive(&s);
    write(
        &d.join("set/2026-01-02.toml"),
        "[e.7]\ndate = \"2026-01-02T10:01:00+00:00\"\nsession = 0\nexercise = 0\nload = 60.0\nrep = 8.0\ndesc = \"Super set Y raise\"\n\n[e.8]\ndate = \"2026-01-02T10:02:00+00:00\"\nsession = 0\nexercise = 0\nload = 60.0\nrep = 6.0\ndesc = \"\"\n",
    );
    let db = gym::db::Db::create(&s.db()).await.unwrap();
    let r = import::import(&db, &d, None).await.unwrap();
    let text = r.to_lines();
    assert!(text.contains("no rep count"), "{text}");
    assert!(text.contains("Y raise"), "{text}");
}
