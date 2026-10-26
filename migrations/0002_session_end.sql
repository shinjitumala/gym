-- A session is open while it is being recorded and closed when it is ended.
-- Rows written before this column existed are all finished sessions, so they
-- close at their own timestamp rather than staying open forever.
ALTER TABLE session ADD COLUMN ended TEXT;

UPDATE session SET ended = date;
