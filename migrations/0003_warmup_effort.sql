-- A warmup set is recorded like any other set, and its effort column says so:
-- `warmup` is a value of the vocabulary, not a rung on the ladder the
-- recommendation compares. A warmup is not taken near failure, so ranking it
-- as an effort would read a limp first set as a regression and a grinder as
-- progress; the code skips the name everywhere it compares or counts sets.
--
-- The rank is outside the three-rung range (fail 0, form fail 1, hard 3) so a
-- row written by something that does not skip it sorts with the easiest sets
-- rather than pretending to be a hard one. It is never read as an effort.
INSERT INTO effort (name, sort) VALUES ('warmup', 7);
