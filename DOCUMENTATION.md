# dead-code-finder Documentation

`dead-code-finder` is a static reachability checker for Python projects. It reports functions, classes, methods, and fields that are not reachable from the configured entrypoints.

## Install

After the package is published:

```bash
pip install dead-code-finder
```

You can also install a wheel built from this repository:

```bash
pip install /path/to/dead_code_finder-0.1.0-py3-none-PLATFORM.whl
```

Or download a release binary and put `dead-code-finder` on your `PATH`.

## Quick Start

From a Python project root:

```bash
dead-code-finder
```

Without a `dead-code-finder.json` file, the tool analyzes the current directory as one Python project. It treats `if __name__ == "__main__"` blocks as production entrypoints and reports symbols that are not reachable from those entrypoints. Test files are not production entrypoints by default.

This default is intentionally simple. It is useful for scripts and small projects, but most applications should add a configuration file so framework routes, package roots, generated clients, and auxiliary scripts are modeled explicitly.

Use JSON output for automation:

```bash
dead-code-finder --format json
```

Make unresolved type diagnostics fail CI:

```bash
dead-code-finder --strict
```

Use a non-default config path:

```bash
dead-code-finder --config tools/dead-code-finder.json
```

## Configuration File

Create `dead-code-finder.json` at the project root.

Minimal single-package config:

```json
{
  "roots": [
    { "path": "src/example_app", "module": "example_app" }
  ],
  "entrypoints": [
    "src/example_app/__main__.py"
  ]
}
```

`roots` tells the analyzer which files belong to the project and what module name each root maps to. Imports are followed only when they resolve inside configured roots or type sources.

For a flat project where the current directory is the import root:

```json
{
  "roots": [
    { "path": ".", "module": "" }
  ]
}
```

With an empty `module`, `main.py` is indexed as `main`, and `pkg/service.py` is indexed as `pkg.service`.

## Monorepos

Use multiple roots for workspaces:

```json
{
  "roots": [
    { "path": "apps/api/src/api", "module": "api" },
    { "path": "packages/shared/src/shared", "module": "shared" }
  ],
  "entrypoints": [
    "apps/api/src/api/main.py"
  ]
}
```

Root globs are supported:

```json
{
  "roots": [
    { "path": "packages/*/src/*", "module": "{basename}" }
  ]
}
```

Each expanded root must produce a unique module name.

## Entrypoints

The default root group is `main`. These symbols count as used:

```json
{
  "roots": [{ "path": "src/example_app", "module": "example_app" }],
  "entrypoints": [
    "src/example_app/main.py",
    "src/example_app/cli.py"
  ]
}
```

Files matched by `entrypoints` are treated as production roots. A file with an `if __name__ == "__main__"` block is also a root.

## Root Groups

Use `rootGroups` when you want separate reports for production, scripts, tests, or other categories.

```json
{
  "roots": [{ "path": "src/example_app", "module": "example_app" }],
  "rootGroups": [
    {
      "name": "production",
      "entrypoints": ["src/example_app/main.py"],
      "countsAsUsed": true
    },
    {
      "name": "scripts",
      "entrypoints": ["src/example_app/scripts/**/*.py"],
      "countsAsUsed": true
    },
    {
      "name": "tests",
      "entrypoints": ["tests/**/*.py"],
      "countsAsUsed": true
    }
  ]
}
```

`countsAsUsed` controls whether reachability from that group suppresses normal dead-code findings. Set it to `true` for code paths that are valid surfaces, such as maintained scripts or tests. Set it to `false` when you want a separate reachability view without keeping production symbols alive.

If `rootGroups` is omitted:

- `entrypoints` become the `main` group.
- `weakEntrypoints` become a `weak` group with `countsAsUsed: false`.
- `includeTests: true` adds a `test` group using `testPatterns`.

## Tests

Tests are not production entrypoints by default. This means test-only usage does not keep production code alive.

To include tests as their own root group:

```json
{
  "roots": [{ "path": "src/example_app", "module": "example_app" }],
  "includeTests": true
}
```

Default test patterns:

```json
[
  "test_*.py",
  "*_test.py",
  "*_test_*.py",
  "tests/**",
  "conftest.py"
]
```

Override them with `testPatterns` if your project uses different naming.

## Type Sources

Use `typeSources` for importable Python or stub packages that should provide type information but should not be reported for dead code:

```json
{
  "roots": [{ "path": "src/example_app", "module": "example_app" }],
  "typeSources": [
    { "path": ".venv/lib/python3.12/site-packages/typedlib", "module": "typedlib" },
    { "path": "stubs/typedlib", "module": "typedlib" }
  ]
}
```

This is useful for local typed libraries, checked-in stubs, or selected packages in a virtual environment.

## Framework Rules

Framework behavior is modeled with declarative rules. The analyzer does not assume built-in support for every web framework, ORM, task runner, or validation library.

FastAPI-style example:

```json
{
  "roots": [{ "path": "src/example_app", "module": "example_app" }],
  "entrypoints": ["src/example_app/main.py"],
  "rules": {
    "constructors": [
      {
        "match": "fastapi.APIRouter",
        "producesType": "fastapi.APIRouter"
      }
    ],
    "decorators": [
      {
        "receiverType": "fastapi.APIRouter",
        "methods": ["get", "post", "put", "patch", "delete"],
        "effect": "registerDecoratedFunction"
      }
    ],
    "calls": [
      {
        "function": "fastapi.Depends",
        "effect": "useCallableArgument",
        "argument": 0
      },
      {
        "receiverType": "fastapi.FastAPI",
        "method": "include_router",
        "effect": "connectRouter",
        "argument": 0
      }
    ],
    "routeGlobs": [
      {
        "whenFunctionCalled": "api.routes.loader.include_routes",
        "glob": "src/example_app/routes/**/*.py",
        "export": "router",
        "effect": "includeRouter"
      }
    ]
  }
}
```

Common rule categories:

- `constructors`: map a constructor or factory symbol to a produced receiver type.
- `decorators`: mark decorated functions as registered entrypoints or model decorator wrappers.
- `calls`: model framework calls that consume callables, connect routers, or use a member of an argument.
- `factoryReturns`: model generic factory functions whose type arguments describe input or output model surfaces.
- `classSurfaces`: mark class-level attributes for classes inheriting from a configured base.
- `assignments`: model assignment-based overrides such as test dependency replacement.
- `fluentMethods`: preserve receiver type through fluent external methods.
- `routeGlobs`: model dynamic module loading from explicit route-loader calls.

Unsupported or misspelled rule effects are configuration errors.

## Allow Comments

Use allow comments when code is intentionally kept even though it is not reachable from normal entrypoints.

File scope:

```python
# dead-code-finder: allow
```

Class or function scope:

```python
# dead-code-finder: allow
class GeneratedClient:
    ...
```

Single-line enum member or field:

```python
class ExampleKind(StrEnum):
    OLD_VALUE = "old_value"  # dead-code-finder: allow
```

An allow comment acts as an explicit root. If an allowed function or class uses helpers, those helpers are also considered reachable.

## Exit Codes

- `0`: no findings, and no strict diagnostics failure.
- `1`: findings were emitted, or `--strict` was used and diagnostics were emitted.
- `2`: command-line, configuration, parsing, or output error.
- `141`: output pipe was closed, such as `dead-code-finder | head`.

## Practical Rollout

Start with:

```bash
dead-code-finder --format text
```

Then add `dead-code-finder.json` once the default root is too coarse. Configure roots first, then production entrypoints, then framework rules. Use `typeSources` for typed local or third-party libraries, and use allow comments for generated or intentionally retained surfaces.
