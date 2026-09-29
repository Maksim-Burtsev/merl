/* An X-macro table: whoever includes it defines TALLY_ROW first. */
struct fruit_counts {
    TALLY_ROW(apples)
    //^ d: src/tally.c:1
    TALLY_ROW(pears)
};
