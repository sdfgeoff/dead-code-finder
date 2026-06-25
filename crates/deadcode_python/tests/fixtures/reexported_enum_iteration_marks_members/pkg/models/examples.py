from enum import StrEnum


class ExampleRole(StrEnum):
    SUBMITTER = "submitter"
    CONTACT = "contact"
    EDITOR = "editor"
    TASK_EDITOR = "task_editor"
    INVOICE_RECIPIENT = "invoice_recipient"
    ACCOUNT_HOLDER = "account_holder"
