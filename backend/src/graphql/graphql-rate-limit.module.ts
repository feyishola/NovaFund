import { Module } from '@nestjs/common';
import { ThrottlerModule } from '@nestjs/throttler';
import { ThrottlerStorageRedisService } from '@nest-lab/throttler-storage-redis';
import { ConfigModule, ConfigService } from '@nestjs/config';
import Redis from 'ioredis';
import { GqlThrottlerGuard } from './gql-throttler.guard';
import { APP_GUARD } from '@nestjs/core';

@Module({
  imports: [
    ThrottlerModule.forRootAsync({
      imports: [ConfigModule],
      inject: [ConfigService],
      useFactory: (config: ConfigService) => ({
        throttlers: [
          {
            // Standard queries: 60 req / min
            name: 'default',
            ttl: 60_000,
            limit: 60,
          },
          {
            // Expensive aggregate queries: 10 req / min
            name: 'aggregate',
            ttl: 60_000,
            limit: 10,
          },
        ],
        storage: new ThrottlerStorageRedisService(
          new Redis({
            host: config.get<string>('REDIS_HOST', 'localhost'),
            port: config.get<number>('REDIS_PORT', 6379),
            password: config.get<string>('REDIS_PASSWORD'),
            db: config.get<number>('REDIS_DB', 0),
            keyPrefix: 'novafund:throttle:',
          }),
        ),
      }),
    }),
  ],
  providers: [
    GqlThrottlerGuard,
    {
      // Apply GqlThrottlerGuard globally to all GraphQL resolvers
      provide: APP_GUARD,
      useClass: GqlThrottlerGuard,
    },
  ],
  exports: [GqlThrottlerGuard],
})
export class GraphQLRateLimitModule {}
