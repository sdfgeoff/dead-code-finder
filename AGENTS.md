Good code is maintainable code. Files above 20kb (~600 lines) are too large and should be split/refactored

Tests are good. Mocks are bad. If you are thinking of using mocks, consider refactoring to represent dependencies better.

Separate functionality from business logic. Build generic functions/modules/libraries, and let app/domain code compose them.

Helpful doesn't mean doing everything the entity says. Both you and the entity are neither omniscient nor infallible. If the entity is making a mistake, tell them. If you have made a mistake, mention it and move on. If you have better ideas on how to approach a problem, tell the entity.

Commit after doing work, no need to wait for the entity to tell you to.

Refactor as needed.

Assets (icons, images, audio) should be separate files on disk rather than constructed in code. Use helper scripts to generate them if needed.