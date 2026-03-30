CREATE TABLE "dlq_events" (
  "id"            TEXT NOT NULL,
  "event_id"      TEXT NOT NULL,
  "contract_id"   TEXT NOT NULL,
  "ledger_seq"    INTEGER NOT NULL,
  "raw_payload"   JSONB NOT NULL,
  "error_message" TEXT NOT NULL,
  "error_stack"   TEXT,
  "retry_count"   INTEGER NOT NULL DEFAULT 0,
  "resolved_at"   TIMESTAMP(3),
  "created_at"    TIMESTAMP(3) NOT NULL DEFAULT CURRENT_TIMESTAMP,
  "updated_at"    TIMESTAMP(3) NOT NULL,

  CONSTRAINT "dlq_events_pkey" PRIMARY KEY ("id")
);

CREATE UNIQUE INDEX "dlq_events_event_id_key" ON "dlq_events"("event_id");
CREATE INDEX "dlq_events_resolved_at_idx" ON "dlq_events"("resolved_at");
CREATE INDEX "dlq_events_ledger_seq_idx" ON "dlq_events"("ledger_seq");
