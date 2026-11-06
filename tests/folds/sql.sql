-- Every construct `f` folds in SQL, and the cases that could break it. -- f: none
CREATE TABLE person ( -- f: 2-11
    id serial PRIMARY KEY, -- f: 2-11
    name text NOT NULL DEFAULT 'a ; and a ( in a string', -- f: 2-11
    -- a comment with a ; and a ( -- f: 2-11
    bio text DEFAULT 'it''s; still a string', -- f: 2-11
    "odd;name" int, -- f: 2-11
    /* a block comment; -- f: 2-11
       over two lines ( */ -- f: 2-11
    created timestamp DEFAULT now() -- f: 2-11
); -- f: 2-11

SELECT 1; -- f: none

ALTER TABLE person -- f: 15-16
    ADD COLUMN age int; -- f: 15-16

CREATE VIEW person_view AS -- f: 18-42
WITH adults AS ( -- f: 18-42
    SELECT -- f: 20-25
        * -- f: 20-25
    FROM -- f: 20-25
        person -- f: 20-25
    WHERE -- f: 20-25
        age >= 18 -- f: 20-25
), -- f: 18-42
named AS MATERIALIZED ( -- f: 18-42
    SELECT -- f: 28-31
        name -- f: 28-31
    FROM -- f: 28-31
        adults) -- f: 28-31
SELECT -- f: 18-42
    p.name, -- f: 18-42
    ( -- f: 18-42
        SELECT -- f: 18-42
            count(*) -- f: 18-42
        FROM -- f: 18-42
            person q -- f: 18-42
        WHERE -- f: 18-42
            q.age = p.age) AS peers -- f: 18-42
FROM -- f: 18-42
    named p; -- f: 18-42

CREATE OR REPLACE FUNCTION touch () -- f: 44-57
    RETURNS TRIGGER -- f: 44-57
    LANGUAGE plpgsql -- f: 44-57
    AS $$ -- f: 44-57
BEGIN -- f: 44-57
    UPDATE -- f: 49-54
        person -- f: 49-54
    SET -- f: 49-54
        name = 'a $$ in a string' -- f: 49-54
    WHERE -- f: 49-54
        id = NEW.id; -- f: 49-54
    RETURN NULL; -- f: 44-57
END -- f: 44-57
$$; -- f: 44-57

CREATE FUNCTION first_name (n int) -- f: 59-69
    RETURNS text -- f: 59-69
    LANGUAGE sql -- f: 59-69
    AS $body$ -- f: 59-69
    SELECT -- f: 63-68
        name -- f: 63-68
    FROM -- f: 63-68
        person -- f: 63-68
    WHERE -- f: 63-68
        id = $1 -- f: 63-68
$body$; -- f: 59-69

WITH RECURSIVE countdown (n) AS ( -- f: 71-84
    SELECT -- f: 72-80
        3 -- f: 72-80
    UNION ALL -- f: 72-80
    SELECT -- f: 72-80
        n - 1 -- f: 72-80
    FROM -- f: 72-80
        countdown -- f: 72-80
    WHERE -- f: 72-80
        n > 0) -- f: 72-80
SELECT -- f: 71-84
    n -- f: 71-84
FROM -- f: 71-84
    countdown; -- f: 71-84

UPDATE -- f: 86-95
    person -- f: 86-95
SET -- f: 86-95
    bio = E'a \' quote; still the string' -- f: 86-95
WHERE -- f: 86-95
    id IN ( -- f: 86-95
        SELECT -- f: 86-95
            id -- f: 86-95
        FROM -- f: 86-95
            person); -- f: 86-95

INSERT INTO person (name) -- f: 97-98
    VALUES ('one'), ('two'); -- f: 97-98
DELETE FROM person -- f: 99-100
WHERE id = 1; -- f: 99-100

-- PL/pgSQL control lines fold nothing of their own. -- f: none
CREATE FUNCTION tidy () -- f: 103-121
    RETURNS void -- f: 103-121
    LANGUAGE plpgsql -- f: 103-121
    AS $$ -- f: 103-121
DECLARE -- f: 103-121
    r record; -- f: 103-121
BEGIN -- f: 103-121
    IF (SELECT count(*) FROM person) > 10 THEN -- f: 103-121
        DELETE FROM person -- f: 111-112
        WHERE age IS NULL; -- f: 111-112
    ELSE -- f: 103-121
        UPDATE person -- f: 114-115
        SET age = 0; -- f: 114-115
    END IF; -- f: 103-121
    FOR r IN SELECT id FROM person LOOP -- f: 103-121
        RAISE NOTICE '%', r.id; -- f: 103-121
    END LOOP; -- f: 103-121
END -- f: 103-121
$$; -- f: 103-121
