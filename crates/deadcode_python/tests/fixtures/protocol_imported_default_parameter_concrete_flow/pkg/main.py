from pkg.context import EXAMPLE_CONTEXT, PromptContext


def setup(frontend_context: PromptContext) -> str:
    return frontend_context.to_context_prompt()


def orchestrate(*, frontend_context: PromptContext) -> str:
    return setup(frontend_context)


def run(*, user_text: str, frontend_context: PromptContext = EXAMPLE_CONTEXT) -> str:
    return orchestrate(frontend_context=frontend_context)


run(user_text="hello")
