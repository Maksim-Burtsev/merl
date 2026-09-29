declare const socket: { on(name: string, f: unknown): void };

socket.on(
  "documents.delete",
  action((event: string) => {
    return event;
  })
);

socket.on(
  "documents.update",
  action((event: string) => {
    return event;
  })
);
