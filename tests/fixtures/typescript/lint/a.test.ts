// A call that passes a callback declares nothing (#343).
suite("a", () => {
  it("works", async () => {
    expect(1).toBe(1);
  });
});
