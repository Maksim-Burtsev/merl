import math
import time

from starlette.middleware.base import BaseHTTPMiddleware, RequestResponseEndpoint
from starlette.requests import Request
from starlette.responses import JSONResponse, Response
from starlette.types import ASGIApp

from bucket import TokenBucket
from plans import LimitsConfig
from store import BucketStore


class RateLimitMiddleware(BaseHTTPMiddleware):
    def __init__(self, app: ASGIApp, store: BucketStore, config: LimitsConfig) -> None:
        super().__init__(app)
        self.store = store
        self.config = config

    async def dispatch(self, request: Request, call_next: RequestResponseEndpoint) -> Response:
        api_key = request.headers.get("x-api-key")
        if api_key is None:
            return JSONResponse({"error": "missing API key"}, status_code=401)

        limits = self.config.for_key(api_key)
        now = time.time()
        bucket = await self.store.get(api_key) or TokenBucket.for_plan(limits, now)
        allowed = bucket.take(now)
        await self.store.set(api_key, bucket, ttl=limits.idle_ttl())

        if not allowed:
            wait = bucket.retry_after(now)
            return JSONResponse(
                {"error": "rate limit exceeded"},
                status_code=429,
                headers={"Retry-After": str(math.ceil(wait))},
            )
        return await call_next(request)
