-- remittances base ----------------------------------------------------
CREATE TABLE remittances (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    merchant_id           TEXT NOT NULL,
    profile_id            TEXT NOT NULL,
    amount                BIGINT NOT NULL,        -- minor units
    source_currency       TEXT NOT NULL,
    destination_currency  TEXT NOT NULL,
    source_amount         BIGINT,
    destination_amount    BIGINT,
    exchange_rate         NUMERIC,                -- tasa 1→X
    reference             TEXT NOT NULL,
    purpose               TEXT,
    status                TEXT NOT NULL,
    failure_reason        TEXT,

    sender_details        JSONB NOT NULL,         -- estructuras normalizadas en app
    beneficiary_details   JSONB NOT NULL,

    return_url            TEXT,
    metadata              JSONB,

    connector             TEXT NOT NULL,
    client_secret         TEXT,
    remittance_date       DATE NOT NULL,

    created_at            TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at            TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- payment -------------------------------------------------------------
CREATE TABLE remittance_payments (
    remittance_id      UUID PRIMARY KEY REFERENCES remittances(id) ON DELETE CASCADE,
    payment_id         TEXT,
    connector_txn_id   TEXT,
    status             TEXT,
    auth_type          TEXT,
    created_at         TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at         TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- payout ---------------------------------------------------------------
CREATE TABLE remittance_payouts (
    remittance_id      UUID PRIMARY KEY REFERENCES remittances(id) ON DELETE CASCADE,
    payout_id          TEXT,
    connector_txn_id   TEXT,
    status             TEXT,
    remittance_method  TEXT,              -- bank, wallet, card…
    created_at         TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at         TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE INDEX idx_remittances_merchant ON remittances(merchant_id);
CREATE INDEX idx_remittances_status   ON remittances(status);
