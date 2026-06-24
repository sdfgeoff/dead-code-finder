# Python Dead Code Finder Specification

## Purpose

This project is a static dead-code checker for Python codebases. It is intended to provide a Rust/Clippy-like reachability analysis for Python projects: functions, classes, methods, and fields should be reported as unused when they are not reachable from configured entrypoints.

The tool should be materially more accurate than name-based scanners such as Vulture. In particular, attribute and method use must be tied to the resolved receiver type. A reference to `.some_field_name` on one object must not keep alive every field with that name on every model.

The tool is not a general-purpose Python linter. Existing tools such as Ruff already handle local unused variables and unused imports well enough. This checker focuses on cross-file and type-aware dead code.

## Core Assumptions

The primary target is Python code that is written in a typed, Pyright-strict style.

The analyzer assumes:

- no dynamic imports in normal application code,
- no meaningful symbol access through string names,
- no metaprogramming that creates or calls local symbols invisibly,
- no fallback from unresolved type information to text or name matching.

Project-specific dynamic patterns may still be modeled through declarative rules. For example, a route loader that imports every `route.py` under a directory can be described as a rule rather than treated as arbitrary importlib behavior.

## Non-Goals

The first version does not need to:

- lint local unused variables,
- lint unused imports,
- report whole dead files as a primary finding,
- infer arbitrary runtime behavior,
- execute Python code,
- depend on Pyright configuration,
- provide built-in Rust plugins for every framework.

Whole-file/module deadness is useful, but it is not a v1 reporting goal. The analyzer may model module top-level reachability internally where needed.

## Definition Of Used

A symbol is used if it is reachable from the active root set through statically resolved graph edges.

A symbol is unused if no active root set reaches it.

Reachability is more important than reference counting. If `old_view()` calls `old_helper()`, but no entrypoint reaches `old_view()`, both are dead. `old_helper()` is not considered alive merely because dead code references it.

## Root Sets

The analyzer builds one symbol graph, then runs reachability from one or more root sets.

Initial root sets:

- `main`: configured entrypoints, `if __name__ == "__main__"` blocks, console-style configured entrypoints, and configured framework roots.
- `test`: pytest tests, fixtures, and test support code when test analysis is enabled.
- `weak`: configured auxiliary entrypoints, such as one-off scripts, that should be analyzed without keeping production symbols alive.

Tests and weak entrypoints are not part of the main root set by default. Their references must not keep production symbols alive.

The report should eventually distinguish:

- dead everywhere,
- dead from main roots but reachable from tests,
- dead from main roots but reachable from weak entrypoints,
- dead inside the test root set.

## Entry Points

Default entrypoint behavior:

- `if __name__ == "__main__"` blocks are roots.
- Explicit configured entrypoints are roots.
- Files matched by configured weak entrypoints are roots only for the weak root set, even when they contain `if __name__ == "__main__"`.
- Framework entrypoints are roots only when modeled through declarative rules.
- Tests are roots only for the test root set, not for main reachability.

Public naming convention alone does not make a symbol alive. A non-underscore function is not automatically public.

`__all__` defines export surface but does not automatically make exported symbols alive. It matters when a downstream import uses that export surface, such as `from module import *`, or when a project explicitly configures library export roots.

## Analysis Boundary

The analyzer works over configured Python roots. It must support multiple roots in one run, including monorepo layouts such as:

```json
{
  "roots": [
    { "path": "example_app", "module": "example_app" },
    { "path": "packages/*/src/*", "module": "{basename}" }
  ]
}
```

Root globs expand deterministically. If two roots claim the same module name, configuration validation fails.

The tool must not read `pyrightconfig.json` as its source of truth. It should have its own JSON configuration format. It may eventually provide an init command that inspects `pyproject.toml` or common layouts and writes explicit config, but runtime behavior should be driven by the tool's own config.

Imports are followed only when they resolve inside configured roots. External imports are represented as opaque nominal identities, such as `fastapi.APIRouter`.

## Module And Import Semantics

Modules have synthetic top-level execution nodes.

Importing a module creates an edge to the imported module's top-level execution node. It does not make every function/class defined in that module alive.

`from module import symbol` creates a binding. The imported symbol is used only when that binding is used downstream.

Imports in unreachable modules do not keep imported modules or symbols alive. An import edge only matters if the importing top-level or code path is reachable.

## Symbol Kinds

V1 should analyze:

- module-level functions,
- classes,
- instance methods,
- class methods,
- static methods,
- class attributes,
- instance attributes when ownership can be resolved,
- dataclass/Pydantic-style fields.

V1 should not focus on:

- local variables,
- unused imports,
- local-only assignments.

Enum members may be added later.

## Attribute And Field Ownership

Class-owned symbols are discovered from:

- assignments inside class bodies,
- annotations inside class bodies,
- `self.x = ...` assignments inside resolved class methods,
- `self.x: T = ...` annotations inside resolved class methods,
- framework/model rules that define field semantics.

Examples:

```python
class ExampleEntity:
    kind = "human"       # ExampleEntity.kind
    name: str            # ExampleEntity.name

    def __init__(self):
        self.field_text = ""  # ExampleEntity.field_text
```

An external write such as `obj.q = 1` only targets an existing/resolved attribute when `obj` has a resolved type. It must not define or keep alive every `q` in the project.

## Access Kinds

The graph should track access kind separately:

- read,
- write,
- construct,
- call,
- serialize,
- validate.

For initial deletion semantics, any reachable read/write/construct/call can count as use. Later reports can distinguish write-only or serialization-only fields.

Explicit constructor keyword arguments count as construct/write use:

```python
ExampleEntity(name="A")
```

This marks `ExampleEntity.name` used.

V1 should not expand arbitrary `**payload` into field uses. If `ExampleEntity(**payload)` is reachable and field-level accuracy matters, the analyzer may emit a type-resolution or unsupported-construct diagnostic, but it must not guess field names.

## Receiver Type Resolution

Method and attribute use must resolve the receiver type.

This is a core requirement. Any `.method_name()` or `.field_name` usage that cannot be tied to a concrete or nominal receiver type must not keep symbols alive by name.

Supported v1 inference:

- annotated parameters,
- annotated returns,
- constructor assignments,
- `self` and `cls`,
- class field annotations,
- direct imports and aliases.

Examples:

```python
def f(entity: ExampleEntity):
    entity.save()  # resolves to ExampleEntity.save

entity = ExampleEntity(name="A")
entity.save()      # resolves to ExampleEntity.save

def get_user() -> ExampleEntity:
    ...

entity = get_user()
entity.save()      # resolves to ExampleEntity.save
```

Missing annotations are allowed, but unresolved receiver identity produces diagnostics. The analyzer should not infer return types from arbitrary function bodies in v1.

## Unresolved Type Information

Unresolved type information is an analyzer diagnostic, not a dead-code confidence level.

The report has two channels:

- dead-code findings,
- type-resolution diagnostics.

If a reachable call `x.save()` has unknown receiver type, the analyzer emits a warning diagnostic. It does not mark any `save` method alive.

Unused findings remain binary. They should not be annotated with confidence. If unresolved diagnostics exist, the CLI can print a summary warning that graph coverage may be incomplete.

Default exit behavior:

- unused findings fail the run,
- unresolved type diagnostics warn but do not fail,
- invalid config, parse infrastructure errors, and unresolved configured rules fail,
- strict mode can make unresolved type diagnostics fail.

## Inheritance And Virtual Dispatch

The analyzer must support Python MRO over known local classes.

If a subclass does not override a method, a call through the subclass keeps the inherited implementation alive.

If a subclass overrides a method, the override is a distinct symbol.

For base-typed values, the analyzer should track concrete subtype flow:

```python
def process(repo: Repository):
    repo.save()

process(SqlRepository())
```

The call marks the `Repository.save` slot invoked, and because `SqlRepository` flows into a `Repository`-typed value, `SqlRepository.save` is reachable.

V1 dataflow should cover:

- direct constructor arguments,
- simple local assignment followed by direct call,
- explicit return annotations where available.

It should not require full whole-program points-to analysis at the start.

## Generics

The target codebase uses generics enough that the model must eventually preserve them.

V1 should preserve generic aliases rather than fully solve Python's type system:

- `Feature[int, Polygon, Props]` is represented as base `Feature` plus substitutions.
- Method lookup mostly resolves against the base type.
- Direct field and return substitutions can be applied when straightforward.
- Observed concrete generic instantiations are tracked.

Full type-variable substitution through inheritance and complex method signatures can come later.

## Declarative Rules

Framework behavior should be modeled by declarative JSON rules, not Rust plugins in v1.

Rules operate on fully resolved identities or declared external identities. Textual matching such as `router.get` is not allowed as a fallback.

Example rule shape:

```json
{
  "rules": {
    "constructors": [
      {
        "match": "fastapi.FastAPI",
        "producesType": "fastapi.FastAPI"
      },
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
        "receiverType": "fastapi.FastAPI",
        "method": "include_router",
        "effect": "connectRouter",
        "argument": 0
      },
      {
        "function": "fastapi.Depends",
        "effect": "useCallableArgument",
        "argument": 0
      }
    ],
    "routeGlobs": [
      {
        "whenFunctionCalled": "example_app.route.all_routes.add_all_routes",
        "glob": "example_app/route/api/**/route.py",
        "export": "router",
        "effect": "includeRouter"
      }
    ]
  }
}
```

The rule engine should be generic. FastAPI and Pydantic are proving cases, not special hardcoded framework behavior.

## External Symbols

External packages are not parsed. Their symbols are represented by opaque nominal identities derived from import paths.

Example:

```python
from fastapi import APIRouter
router = APIRouter()
```

The resolver treats `fastapi.APIRouter` as an external constructor if config declares that identity. The constructed value can then have nominal type `fastapi.APIRouter`, allowing a declarative decorator rule to resolve `router.get`.

## Pydantic And Model Semantics

Pydantic-like model support should be expressible through declarative rules.

Initial semantics:

- subclasses of configured model base classes have fields from annotations,
- explicit constructor keyword arguments use matching fields,
- validation/serialization APIs can create validate/serialize access edges where statically resolvable.

The analyzer should not treat a string key or a same-named attribute on another object as use of a model field.

## FastAPI Semantics

FastAPI support should be expressed through declarative rules.

Required patterns:

- `FastAPI()` creates an app object,
- `APIRouter()` creates a router object,
- `@app.get(...)` and `@router.get(...)` register decorated functions,
- `app.include_router(router)` connects router endpoints to the app,
- `Depends(callable)` marks the dependency callable or class as reachable,
- project-specific route glob rules can model structured dynamic route loaders.

## Reporting

The primary entity experience should look like a linter/type checker:

```text
example_app/logic/user_logic.py:42:5 DCF003 unused method ExampleService.delete_user
example_app/foo.py:10:12 DCF101 cannot resolve receiver type for repo.save
```

Reports should not include paths explaining why live symbols are used. That would expose too much graph/AST detail for the normal linting workflow.

The library should return structured report data with:

- findings,
- diagnostics,
- source spans,
- rule/error codes,
- severities,
- summary counts,
- root-set labels.

The CLI should support text output first and JSON output as a machine-readable rendering of the same report.

Suggested codes:

- `DCF001`: unused function,
- `DCF002`: unused class,
- `DCF003`: unused method,
- `DCF004`: unused attribute or field,
- `DCF101`: unresolved receiver type,
- `DCF102`: unresolved import or parse diagnostic,
- `DCF103`: unsupported dynamic import,
- `DCF201`: invalid config rule.

## Implementation Architecture

The project should be Rust-first and library-first.

Suggested crates:

- `deadcode_core`: symbol IDs, graph model, diagnostics, findings, reports.
- `deadcode_python`: Python parsing, module resolution, symbol extraction, type and receiver resolution.
- `deadcode_cli`: config loading, filesystem walking, output formatting, exit codes.

The CLI should remain thin. The analysis algorithm should be reusable by other tools.

Parser selection should be isolated behind a module boundary. Ruff's parser is preferred because Ruff has already solved high-speed Python parsing at scale, but the analyzer should not leak parser-specific details through public APIs unnecessarily.

## Testing Strategy

Tests should be fixture-driven and integration-heavy. Mocks should be avoided where a real synthetic project can be analyzed.

Important fixture families:

- single-root packages,
- multi-root workspaces,
- `packages/*/src/*` layouts,
- relative imports,
- dead islands,
- `__main__` scripts,
- FastAPI route decorators,
- router inclusion,
- project-specific route glob loading,
- Pydantic model construction,
- typed service-example_item method calls,
- context managers using `__enter__` and `__exit__`,
- unresolved receiver diagnostics,
- inheritance and overrides,
- generic aliases,
- test-only reachability.

The repository should include synthetic fixture/fuzzer tests that exercise the dominant patterns of the intended real-world target project without depending on that private project.

Fuzz-style tests should be deterministic and produce reproducible, readable fixture code on failure.

## Staged Delivery

The staged plan is:

1. Scaffold Rust workspace and CLI.
2. Add JSON config and root discovery.
3. Parse Python and build a symbol index.
4. Resolve static imports and module names.
5. Add reachability and basic unused reports.
6. Add receiver type resolution.
7. Add attribute/field access semantics.
8. Add inheritance and virtual slot reachability.
9. Add minimal generic type preservation.
10. Add declarative rule engine.
11. Prove FastAPI and Pydantic through declarative fixtures.
12. Add realistic fixture and fuzzer coverage.
13. Polish report formatting and exit semantics.
14. Add test root-set separation.

Each stage should produce used, tested functionality. Avoid adding abstractions that are not exercised by that stage or its tests.
