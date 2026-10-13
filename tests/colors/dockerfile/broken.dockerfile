FROM alpine:3.20 AS
RUN echo "unclosed
ENV =oops
COPY --from= /a /b
CMD ["node",
EXPOSE abc
RUN <<EOF
  echo half
