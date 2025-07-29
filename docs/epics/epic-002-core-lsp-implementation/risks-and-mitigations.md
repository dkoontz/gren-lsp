# Risks and Mitigations
- **Risk:** tree-sitter-gren grammar incomplete
  - *Mitigation:* Contribute missing features, implement fallbacks
- **Risk:** Performance issues with large files
  - *Mitigation:* Implement aggressive caching, optimize queries
- **Risk:** Symbol resolution complexity
  - *Mitigation:* Start with simple cases, iterate on accuracy
