import logging
import structlog


logger = logging.getLogger(__name__)
struct_logger = structlog.get_logger(__name__)


def run_logging_flow():
    logger.info("event")


def run_structlog_flow():
    log = struct_logger.bind(conversation_id=1)
    log.info("event")
