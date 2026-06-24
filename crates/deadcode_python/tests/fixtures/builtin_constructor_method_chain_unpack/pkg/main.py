def decode(token: str) -> bytes:
    message, encoded_signature = str(token).rsplit(".", 1)
    return message.encode("utf-8") + encoded_signature.encode("utf-8")


decode("header.signature")
