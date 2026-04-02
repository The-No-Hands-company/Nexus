-- Enforce privacy-first monetization invariants at the database layer.
-- This prevents bypasses through direct SQL writes or future code paths.

ALTER TABLE marketplace_monetization
    ADD CONSTRAINT chk_marketplace_monetization_consistency
    CHECK (
        (
            is_monetized = FALSE
            AND (price_cents IS NULL OR price_cents <= 0)
        )
        OR
        (
            is_monetized = TRUE
            AND price_cents IS NOT NULL
            AND price_cents > 0
            AND payment_link IS NOT NULL
            AND payment_link ~ '^https?://'
        )
    );

ALTER TABLE marketplace_monetization
    ADD CONSTRAINT chk_marketplace_monetization_purchase_count_non_negative
    CHECK (purchase_count >= 0);
