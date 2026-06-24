from collections.abc import Sequence

from pkg.sdk_alias import StreamChunk


def consume(chunks: Sequence[StreamChunk]):
    for chunk in chunks:
        choice = chunk.choices[0]
        delta = choice.delta
        if delta.content:
            return delta.content
    return None


consume([])
