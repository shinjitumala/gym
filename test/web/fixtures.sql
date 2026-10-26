-- A fixture log for the web scenarios: two places, six exercises with muscle
-- groups, two gym presets, four bench sessions one of which is a deload, five
-- foods, a meal preset, four meals and nine weigh-ins.
--
-- The dates are absolute on purpose. A scenario that asserts "sorted by
-- recency" needs to know which row came last, and a relative fixture would
-- answer differently on a different day.

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

INSERT INTO exercise_list (name) VALUES ('push'), ('legs');
INSERT INTO exercise_list_item (list, exercise, position)
SELECT l.id, e.id, 0 FROM exercise_list l, exercise e
WHERE l.name = 'push' AND e.name = 'bench press';
INSERT INTO exercise_list_item (list, exercise, position)
SELECT l.id, e.id, 1 FROM exercise_list l, exercise e
WHERE l.name = 'push' AND e.name = 'incline press';
INSERT INTO exercise_list_item (list, exercise, position)
SELECT l.id, e.id, 0 FROM exercise_list l, exercise e
WHERE l.name = 'legs' AND e.name = 'squat';

INSERT INTO phase (kind, start, target_rate, target_rep)
    VALUES ('bulk', '2026-09-01T09:00:00+00:00', 2.0, 8);

-- Three bench sessions, each ended the evening it started. The effort gets
-- harder in the middle session without the load or the reps moving, which is
-- the case the recommendation has to catch.
INSERT INTO session (date, place, ended)
SELECT '2026-10-05T18:00:00+00:00', p.id, '2026-10-05T19:30:00+00:00' FROM place p WHERE p.name = 'home';
INSERT INTO session (date, place, ended)
SELECT '2026-10-12T18:00:00+00:00', p.id, '2026-10-12T19:30:00+00:00' FROM place p WHERE p.name = 'home';
INSERT INTO session (date, place, ended)
SELECT '2026-10-19T18:00:00+00:00', p.id, '2026-10-19T19:30:00+00:00' FROM place p WHERE p.name = 'home';

INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 0, s.date, 60, 8, (SELECT id FROM effort WHERE name = 'hard')
FROM session s, exercise e WHERE s.date LIKE '2026-10-05%' AND e.name = 'bench press';
INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 1, s.date, 60, 8, (SELECT id FROM effort WHERE name = 'hard')
FROM session s, exercise e WHERE s.date LIKE '2026-10-05%' AND e.name = 'bench press';
INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 0, s.date, 60, 8, (SELECT id FROM effort WHERE name = 'form fail')
FROM session s, exercise e WHERE s.date LIKE '2026-10-12%' AND e.name = 'bench press';
INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 0, s.date, 70, 8, (SELECT id FROM effort WHERE name = 'hard')
FROM session s, exercise e WHERE s.date LIKE '2026-10-19%' AND e.name = 'bench press';
INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 1, s.date, 70, 8, (SELECT id FROM effort WHERE name = 'hard')
FROM session s, exercise e WHERE s.date LIKE '2026-10-19%' AND e.name = 'bench press';

-- A deload week: the same exercise lighter, flagged on the session. It is shown
-- in the history and shaded on the charts, but it is not the number to beat,
-- so the session after it compares against the 70 kg one.
INSERT INTO session (date, place, deload, ended)
SELECT '2026-10-16T18:00:00+00:00', p.id, 1, '2026-10-16T19:00:00+00:00' FROM place p WHERE p.name = 'home';
INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 0, s.date, 40, 8, (SELECT id FROM effort WHERE name = 'hard')
FROM session s, exercise e WHERE s.date LIKE '2026-10-16%' AND e.name = 'bench press';
INSERT INTO training_set (session, exercise, position, date, load, rep, effort)
SELECT s.id, e.id, 1, s.date, 40, 8, (SELECT id FROM effort WHERE name = 'hard')
FROM session s, exercise e WHERE s.date LIKE '2026-10-16%' AND e.name = 'bench press';

INSERT INTO food (name, calories, protein, fat, carbohydrate) VALUES
    ('oats', 380, 13, 7, 60),
    ('milk', 120, 6, 5, 10),
    ('rice', 350, 7, 1, 78),
    ('chicken', 200, 40, 4, 0),
    ('bread', 250, 8, 3, 48);
INSERT INTO meal_preset (name, last_used) VALUES ('breakfast', '2026-10-25T08:00:00+00:00');
INSERT INTO meal_preset_item (preset, food, amount)
SELECT p.id, f.id, 1.0 FROM meal_preset p, food f
WHERE p.name = 'breakfast' AND f.name IN ('oats', 'milk');

INSERT INTO meal (date, food, amount)
SELECT '2026-10-24T08:10:00+00:00', id, 1.5 FROM food WHERE name = 'oats';
INSERT INTO meal (date, food, amount)
SELECT '2026-10-24T12:30:00+00:00', id, 1.0 FROM food WHERE name = 'rice';
INSERT INTO meal (date, food, amount)
SELECT '2026-10-24T12:30:00+00:00', id, 2.0 FROM food WHERE name = 'chicken';
INSERT INTO meal (date, food, amount)
SELECT '2026-10-25T08:00:00+00:00', id, 1.0 FROM food WHERE name = 'bread';

INSERT INTO weight (date, kg, bodyfat) VALUES
    ('2026-09-01T07:00:00+00:00', 80.0, 18.0),
    ('2026-09-08T07:00:00+00:00', 80.4, 17.8),
    ('2026-09-15T07:00:00+00:00', 80.6, 17.9),
    ('2026-09-22T07:00:00+00:00', 80.9, 17.7),
    ('2026-09-29T07:00:00+00:00', 81.1, 17.6),
    ('2026-10-06T07:00:00+00:00', 81.2, 17.5),
    ('2026-10-13T07:00:00+00:00', 81.6, 17.4),
    ('2026-10-20T07:00:00+00:00', 81.8, 17.3),
    ('2026-10-24T07:00:00+00:00', 81.7, 17.2);
