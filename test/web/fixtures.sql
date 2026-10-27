-- A fixture log for the web scenarios: two places, six exercises with muscle
-- groups, two gym presets with a working-set target per exercise, four bench
-- sessions one of which is a deload and one of which is only a warmup, two squat
-- sessions and two row sessions (so the analysis has a list to search), five
-- foods, a meal preset, four meals and ten weigh-ins.
--
-- Every date here is counted back from the day the suite runs, never written as
-- a calendar day: each row names a day of `DAYS_AGO` in `drive.py` and the time
-- of day it happened at, and `drive.py` fills the day in before the script is
-- run. A scenario about recency ("yesterday", the range's oldest weigh-in) has
-- to answer the same on any day the suite runs, and a fixture pinned to a date
-- starts reading "2 days ago" the day after it was written.

INSERT INTO place (name) VALUES ('home'), ('the gym');
INSERT INTO muscle_group (name) VALUES ('chest'), ('back'), ('legs');
INSERT INTO exercise (name) VALUES
    ('bench press'), ('squat'), ('row'), ('pull up'), ('incline press'), ('deadlift');
INSERT INTO exercise_muscle_group (exercise, muscle_group, amount)
SELECT e.id, m.id, 1.0 FROM exercise e, muscle_group m
WHERE (e.name, m.name) IN (
    ('bench press', 'chest'), ('incline press', 'chest'),
    ('squat', 'legs'), ('deadlift', 'legs'),
    ('row', 'back'), ('pull up', 'back')
);

-- Two presets, each exercise with the working sets it is there to do. The two
-- counts differ, so a page that shows a single number for the list cannot pass
-- for showing each exercise's own target.
INSERT INTO exercise_list (name) VALUES ('push'), ('legs');
INSERT INTO exercise_list_item (list, exercise, position, target_sets)
SELECT l.id, e.id, 0, 3 FROM exercise_list l, exercise e
WHERE l.name = 'push' AND e.name = 'bench press';
INSERT INTO exercise_list_item (list, exercise, position, target_sets)
SELECT l.id, e.id, 1, 2 FROM exercise_list l, exercise e
WHERE l.name = 'push' AND e.name = 'incline press';
INSERT INTO exercise_list_item (list, exercise, position, target_sets)
SELECT l.id, e.id, 0, 3 FROM exercise_list l, exercise e
WHERE l.name = 'legs' AND e.name = 'squat';

INSERT INTO phase (kind, start, target_rate, target_rep)
    VALUES ('bulk', '{{at:phase_start:09:00:00}}', 2.0, 8);

-- Three bench sessions, each ended the evening it started. The effort gets
-- harder in the middle session without the load or the reps moving, which is
-- the case the recommendation has to catch.
INSERT INTO session (date, place, ended)
SELECT '{{at:bench_first:18:00:00}}', p.id, '{{at:bench_first:19:30:00}}' FROM place p WHERE p.name = 'home';
INSERT INTO session (date, place, ended)
SELECT '{{at:bench_second:18:00:00}}', p.id, '{{at:bench_second:19:30:00}}' FROM place p WHERE p.name = 'home';
INSERT INTO session (date, place, ended)
SELECT '{{at:bench_third:18:00:00}}', p.id, '{{at:bench_third:19:30:00}}' FROM place p WHERE p.name = 'home';

INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 0, s.date, 60, 8, (SELECT id FROM effort WHERE name = 'hard')
FROM session s, exercise e WHERE s.date LIKE '{{on:bench_first}}%' AND e.name = 'bench press';
INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 1, s.date, 60, 8, (SELECT id FROM effort WHERE name = 'hard')
FROM session s, exercise e WHERE s.date LIKE '{{on:bench_first}}%' AND e.name = 'bench press';
INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 0, s.date, 60, 8, (SELECT id FROM effort WHERE name = 'form fail')
FROM session s, exercise e WHERE s.date LIKE '{{on:bench_second}}%' AND e.name = 'bench press';
-- The newest working session opens with a warmup: it is recorded and shown, and
-- it is not one of the session's two working sets nor the set to beat.
INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 0, s.date, 40, 10, (SELECT id FROM effort WHERE name = 'warmup')
FROM session s, exercise e WHERE s.date LIKE '{{on:bench_third}}%' AND e.name = 'bench press';
INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 1, s.date, 70, 8, (SELECT id FROM effort WHERE name = 'hard')
FROM session s, exercise e WHERE s.date LIKE '{{on:bench_third}}%' AND e.name = 'bench press';
INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 2, s.date, 70, 8, (SELECT id FROM effort WHERE name = 'hard')
FROM session s, exercise e WHERE s.date LIKE '{{on:bench_third}}%' AND e.name = 'bench press';

-- A session that was only a warmup. It is the newest bench session, and it is
-- not the numbers to beat: the comparison reaches back past it to the 70 kg
-- session, so an unfinished workout cannot read as a regression.
INSERT INTO session (date, place, ended)
SELECT '{{at:bench_warmup_only:18:00:00}}', p.id, '{{at:bench_warmup_only:18:20:00}}' FROM place p WHERE p.name = 'home';
INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 0, s.date, 40, 10, (SELECT id FROM effort WHERE name = 'warmup')
FROM session s, exercise e WHERE s.date LIKE '{{on:bench_warmup_only}}%' AND e.name = 'bench press';

-- A deload week: the same exercise lighter, flagged on the session. It is shown
-- in the history and shaded on the charts, but it is not the number to beat,
-- so the session after it compares against the 70 kg one.
INSERT INTO session (date, place, deload, ended)
SELECT '{{at:deload:18:00:00}}', p.id, 1, '{{at:deload:19:00:00}}' FROM place p WHERE p.name = 'home';
INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 0, s.date, 40, 8, (SELECT id FROM effort WHERE name = 'hard')
FROM session s, exercise e WHERE s.date LIKE '{{on:deload}}%' AND e.name = 'bench press';
INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 1, s.date, 40, 8, (SELECT id FROM effort WHERE name = 'hard')
FROM session s, exercise e WHERE s.date LIKE '{{on:deload}}%' AND e.name = 'bench press';

-- A second exercise with two sessions: the analysis dropdown lists exercises
-- trained at least twice, and one entry is not a list to search.
INSERT INTO session (date, place, ended)
SELECT '{{at:squat_first:18:00:00}}', p.id, '{{at:squat_first:19:00:00}}' FROM place p WHERE p.name = 'the gym';
INSERT INTO session (date, place, ended)
SELECT '{{at:squat_second:18:00:00}}', p.id, '{{at:squat_second:19:00:00}}' FROM place p WHERE p.name = 'the gym';
INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 0, s.date, 100, 5, (SELECT id FROM effort WHERE name = 'hard')
FROM session s, exercise e WHERE s.date LIKE '{{on:squat_first}}%' AND e.name = 'squat';
INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 0, s.date, 102.5, 5, (SELECT id FROM effort WHERE name = 'hard')
FROM session s, exercise e WHERE s.date LIKE '{{on:squat_second}}%' AND e.name = 'squat';

-- A third exercise worked twice, plus a session that was only its warmup: two
-- sessions of work, so the analysis offers it and draws it, and the warmup-only
-- session is neither a third session nor a bar of volume.
INSERT INTO session (date, place, ended)
SELECT '{{at:row_first:18:00:00}}', p.id, '{{at:row_first:19:00:00}}' FROM place p WHERE p.name = 'the gym';
INSERT INTO session (date, place, ended)
SELECT '{{at:row_second:18:00:00}}', p.id, '{{at:row_second:19:00:00}}' FROM place p WHERE p.name = 'the gym';
INSERT INTO session (date, place, ended)
SELECT '{{at:row_warmup_only:18:00:00}}', p.id, '{{at:row_warmup_only:18:15:00}}' FROM place p WHERE p.name = 'the gym';
INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 0, s.date, 60, 8, (SELECT id FROM effort WHERE name = 'hard')
FROM session s, exercise e WHERE s.date LIKE '{{on:row_first}}%' AND e.name = 'row';
INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 0, s.date, 65, 8, (SELECT id FROM effort WHERE name = 'hard')
FROM session s, exercise e WHERE s.date LIKE '{{on:row_second}}%' AND e.name = 'row';
INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 0, s.date, 40, 12, (SELECT id FROM effort WHERE name = 'warmup')
FROM session s, exercise e WHERE s.date LIKE '{{on:row_warmup_only}}%' AND e.name = 'row';

INSERT INTO food (name, calories, protein, fat, carbohydrate) VALUES
    ('oats', 380, 13, 7, 60),
    ('milk', 120, 6, 5, 10),
    ('rice', 350, 7, 1, 78),
    ('chicken', 200, 40, 4, 0),
    ('bread', 250, 8, 3, 48);
INSERT INTO meal_preset (name, last_used) VALUES ('breakfast', '{{at:preset_used:08:00:00}}');
INSERT INTO meal_preset_item (preset, food, amount)
SELECT p.id, f.id, 1.0 FROM meal_preset p, food f
WHERE p.name = 'breakfast' AND f.name IN ('oats', 'milk');

-- Two days of meals, the newest yesterday: the picker is ordered by the last
-- meal of each food, and the note beside a row says how long ago that was.
INSERT INTO meal (date, food, amount)
SELECT '{{at:meal_older:08:10:00}}', id, 1.5 FROM food WHERE name = 'oats';
INSERT INTO meal (date, food, amount)
SELECT '{{at:meal_older:12:30:00}}', id, 1.0 FROM food WHERE name = 'rice';
INSERT INTO meal (date, food, amount)
SELECT '{{at:meal_older:12:30:00}}', id, 2.0 FROM food WHERE name = 'chicken';
INSERT INTO meal (date, food, amount)
SELECT '{{at:meal_newest:08:00:00}}', id, 1.0 FROM food WHERE name = 'bread';

-- One weigh-in well outside the trend's 21-day window: the slope is fitted over
-- that window, the chart is asked about the whole range, and a page that draws
-- the window instead of the range shows an empty line for a log that has weight
-- in it.
INSERT INTO weight (date, kg, bodyfat) VALUES ('{{at:weigh_outside:07:00:00}}', 78.2, 19.4);

INSERT INTO weight (date, kg, bodyfat) VALUES
    ('{{at:weigh_1:07:00:00}}', 80.0, 18.0),
    ('{{at:weigh_2:07:00:00}}', 80.4, 17.8),
    ('{{at:weigh_3:07:00:00}}', 80.6, 17.9),
    ('{{at:weigh_4:07:00:00}}', 80.9, 17.7),
    ('{{at:weigh_5:07:00:00}}', 81.1, 17.6),
    ('{{at:weigh_6:07:00:00}}', 81.2, 17.5),
    ('{{at:weigh_7:07:00:00}}', 81.6, 17.4),
    ('{{at:weigh_8:07:00:00}}', 81.8, 17.3),
    ('{{at:weigh_9:07:00:00}}', 81.7, 17.2);
