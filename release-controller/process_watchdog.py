import os
import logging
import threading


LOGGER = logging.getLogger(__name__)


class Watchdog:
    """A watchdog class that will terminate the application if it is not reported healthy within a given timeout."""

    def __init__(self, timeout_seconds: int | float) -> None:
        """Initialize the watchdog with a timeout in seconds."""
        self.timeout = timeout_seconds
        self._timer: threading.Timer | None = None

    def _handle_timeout(self) -> None:
        LOGGER.error("Watchdog timeout!  Terminating the application now.")
        # Immediately terminates the process with the given status code,
        # bypassing the usual cleanup process that Python performs when exiting.
        # This means that any cleanup handlers, such as try...finally blocks, with statements,
        # and atexit functions, will not be executed.
        # This ensures that the process terminates immediately.
        os._exit(1)  # pylint: disable=protected-access

    def start(self) -> None:
        """Start the watchdog timer."""
        if self._timer:
            self._timer.cancel()
        self._timer = threading.Timer(self.timeout, self._handle_timeout)
        self._timer.setDaemon(True)  # without this, the app keeps running on Ctrl+C
        self._timer.start()

    def report_healthy(self) -> None:
        """Report that the application is healthy, resetting the watchdog timer."""
        if self._timer:
            self._timer.cancel()
        self.start()
