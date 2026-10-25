-- #1284: NEW and OLD are rows of the table the trigger running the function is on.
CREATE FUNCTION scrub () RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    NEW.rim := OLD.holes;
    --  ^ d: migrations/2024-09-01-000009_sift_colanders/up.sql:11
    --             ^ d: migrations/2024-09-01-000009_sift_colanders/up.sql:4
    WITH new AS (SELECT 1 AS rim) SELECT new.rim FROM new;
    --                                       ^ d: none
    RETURN NEW;
END
$$;
CREATE TRIGGER scrub BEFORE UPDATE OF rim,
    holes ON sifters FOR EACH ROW EXECUTE FUNCTION scrub ();
CREATE OR REPLACE FUNCTION drain () RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    DELETE FROM sifters WHERE id = OLD.id;
    --                                 ^ d: none
    RETURN OLD;
END
$$;
CREATE FUNCTION rinse () RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    NEW.holes := 0;
    --  ^ d: none
    RETURN NEW;
END
$$;
