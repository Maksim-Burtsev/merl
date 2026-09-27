FROM python:3.13-slim AS build
WORKDIR /src
COPY pyproject.toml ./
COPY app ./app
RUN pip wheel --no-deps --wheel-dir /wheels .

FROM python:3.13-slim
WORKDIR /srv
COPY --from=build /wheels /wheels
RUN pip install --no-index /wheels/*.whl && rm -rf /wheels
USER nobody
CMD ["python", "-m", "app.api"]
