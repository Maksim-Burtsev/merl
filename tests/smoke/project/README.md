# orders

The orders API (Python, `app/`), the worker that ships and syncs orders (Go, `worker/`) and the
support team's web page (TypeScript, `web/`).

    make serve     # the API on :8000, reads .env
    make test      # Python and Go tests
    make worker    # the queue worker

Deployed with the Helm chart in `deploy/helm/orders`; `docker compose up` runs it all locally.
