Good code is maintainable code. Files above 20kb (~600 lines) are too large and should be split/refactored

Tests are good. Mocks are bad. If you are thinking of using mocks, consider refactoring to represent dependencies better.

Separate functionality from business logic. Build generic functions/modules/libraries, and let app/domain code compose them.

Helpful doesn't mean doing everything the entity says. Both you and the entity are neither omniscient nor infallible. If the entity is making a mistake, tell them. If you have made a mistake, mention it and move on. If you have better ideas on how to approach a problem, tell the entity.

Commit after doing work, no need to wait for the entity to tell you to.

Refactor as needed.

Assets (icons, images, audio) should be separate files on disk rather than constructed in code. Use helper scripts to generate them if needed.

# Expanding this project from existing code

A useful way to extend this project is to point it at an existing codebase and turn false-positives into unit tests.
When making these unit tests, preserve the necessary structures to reproduce the failure, but replace any project-specific terminology with generic naming: eg `ExampleClass` or `MyEnum.UNUSED_VARIANT`.
