/**
 * DLQ Replay Script
 *
 * Replays unresolved DLQ events through the indexer's event handler.
 *
 * Usage:
 *   npx ts-node -r tsconfig-paths/register src/indexer/scripts/dlq-replay.ts
 *
 * Options (env vars):
 *   DLQ_BATCH_SIZE   — how many events to process per run (default: 50)
 *   DLQ_DRY_RUN      — set to "true" to log without processing (default: false)
 */

import { NestFactory } from '@nestjs/core';
import { AppModule } from '../../app.module';
import { DlqService } from '../services/dlq.service';
import { EventHandlerService } from '../services/event-handler.service';
import { IndexerService } from '../services/indexer.service';
import { Logger } from '@nestjs/common';

const logger = new Logger('DlqReplay');

async function replay() {
  const app = await NestFactory.createApplicationContext(AppModule, { logger: ['error', 'warn', 'log'] });

  const dlq = app.get(DlqService);
  const eventHandler = app.get(EventHandlerService);

  const batchSize = parseInt(process.env.DLQ_BATCH_SIZE ?? '50', 10);
  const dryRun = process.env.DLQ_DRY_RUN === 'true';

  const pending = await dlq.listPending();
  const batch = pending.slice(0, batchSize);

  logger.log(`Found ${pending.length} unresolved DLQ events. Processing ${batch.length}${dryRun ? ' (DRY RUN)' : ''}.`);

  let succeeded = 0;
  let failed = 0;

  for (const dlqEvent of batch) {
    const raw = dlqEvent.rawPayload as any;

    if (dryRun) {
      logger.log(`[DRY RUN] Would replay event ${dlqEvent.eventId}`);
      continue;
    }

    try {
      await eventHandler.processEvent(raw);
      await dlq.markResolved(dlqEvent.eventId);
      logger.log(`Replayed and resolved: ${dlqEvent.eventId}`);
      succeeded++;
    } catch (err) {
      logger.error(`Replay failed for ${dlqEvent.eventId}: ${err.message}`);
      failed++;
    }
  }

  logger.log(`Replay complete — succeeded: ${succeeded}, failed: ${failed}`);
  await app.close();
}

replay().catch((err) => {
  logger.error(err);
  process.exit(1);
});
