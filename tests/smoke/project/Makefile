PY ?= .venv/bin/python

.PHONY: serve test lint worker ci

serve:
	$(PY) -m app.api

test:
	$(PY) -m pytest -q tests
	cd worker && go test ./...

lint:
	$(PY) -m ruff check app tests

worker:
	cd worker && go run .

ci: lint test
