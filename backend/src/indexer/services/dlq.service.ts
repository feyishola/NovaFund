import { Injectable, Logger } from '@nestjs/common';
import * as Sentry from '@sentry/node';
import { PrismaService } from '../../prisma.service';
import { SorobanEvent } from '../types/event-types';

@Injectable()
export class DlqService {
  private readonly logger = new Logger(DlqService.name);

  constructor(private readonly prisma: PrismaService) {}

  async push(event: SorobanEvent, error: Error): Promise<void> {
    this.logger.error(
      `DLQ: storing failed event ${event.id} — ${error.message}`,
      error.stack,
    );

    await this.prisma.dlqEvent.upsert({
      where: { eventId: event.id },
      create: {
        eventId: event.id,
        contractId: event.contractId,
        ledgerSeq: event.ledger,
        rawPayload: event as any,
        errorMessage: error.message,
        errorStack: error.stack,
      },
      update: {
        retryCount: { increment: 1 },
        errorMessage: error.message,
        errorStack: error.stack,
        resolvedAt: null,
      },
    });

    Sentry.captureException(error, {
      tags: { component: 'indexer-dlq', eventId: event.id, contractId: event.contractId },
      extra: { ledgerSeq: event.ledger, rawPayload: event },
    });
  }

  async markResolved(eventId: string): Promise<void> {
    await this.prisma.dlqEvent.update({
      where: { eventId },
      data: { resolvedAt: new Date() },
    });
  }

  async listPending() {
    return this.prisma.dlqEvent.findMany({
      where: { resolvedAt: null },
      orderBy: { createdAt: 'asc' },
    });
  }
}
