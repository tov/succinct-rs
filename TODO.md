To Do:

  - compressed integer vectors
  - serialization
  - balanced parentheses
  - more...

Design questions:

  - How can we properly stack up capabilities like RankSupport and
    SelectSupport?

      - Should we use a borrowing model or an ownership model?

  - How should we parameterize RankSupport and SelectSupport to indicate
    a structure that supports all bool queries, all u8 queries, only 1
    (and not 0) queries, etc?

  - Can UniversalCodes better indicate their domains? In types?

  - What can/should we try to do block-wise rather than bit-wise?

  - How should we account for overflows?
