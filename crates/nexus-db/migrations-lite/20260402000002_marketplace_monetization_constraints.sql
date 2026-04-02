-- Enforce privacy-first monetization invariants at the database layer (SQLite).
-- SQLite does not support adding table CHECK constraints via ALTER TABLE,
-- so use triggers to reject invalid writes.

CREATE TRIGGER IF NOT EXISTS trg_marketplace_monetization_validate_insert
BEFORE INSERT ON marketplace_monetization
FOR EACH ROW
BEGIN
    SELECT CASE
        WHEN NEW.purchase_count < 0 THEN
            RAISE(ABORT, 'purchase_count must be >= 0')
    END;

    SELECT CASE
        WHEN NEW.is_monetized = 1
             AND (NEW.price_cents IS NULL OR NEW.price_cents <= 0) THEN
            RAISE(ABORT, 'paid plugins must set price_cents > 0')
    END;

    SELECT CASE
        WHEN NEW.is_monetized = 1
             AND (NEW.payment_link IS NULL
                  OR (NEW.payment_link NOT LIKE 'http://%' AND NEW.payment_link NOT LIKE 'https://%')) THEN
            RAISE(ABORT, 'paid plugins must set a valid http(s) payment_link')
    END;

    SELECT CASE
        WHEN NEW.is_monetized = 0
             AND NEW.price_cents IS NOT NULL
             AND NEW.price_cents > 0 THEN
            RAISE(ABORT, 'free plugins cannot have positive price_cents')
    END;
END;

CREATE TRIGGER IF NOT EXISTS trg_marketplace_monetization_validate_update
BEFORE UPDATE ON marketplace_monetization
FOR EACH ROW
BEGIN
    SELECT CASE
        WHEN NEW.purchase_count < 0 THEN
            RAISE(ABORT, 'purchase_count must be >= 0')
    END;

    SELECT CASE
        WHEN NEW.is_monetized = 1
             AND (NEW.price_cents IS NULL OR NEW.price_cents <= 0) THEN
            RAISE(ABORT, 'paid plugins must set price_cents > 0')
    END;

    SELECT CASE
        WHEN NEW.is_monetized = 1
             AND (NEW.payment_link IS NULL
                  OR (NEW.payment_link NOT LIKE 'http://%' AND NEW.payment_link NOT LIKE 'https://%')) THEN
            RAISE(ABORT, 'paid plugins must set a valid http(s) payment_link')
    END;

    SELECT CASE
        WHEN NEW.is_monetized = 0
             AND NEW.price_cents IS NOT NULL
             AND NEW.price_cents > 0 THEN
            RAISE(ABORT, 'free plugins cannot have positive price_cents')
    END;
END;
