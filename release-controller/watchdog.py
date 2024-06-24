import os
import threading


class Watchdog:
    """A watchdog class that will terminate the application if it is not reported healthy within a given timeout."""

    def __init__(self, timeout_seconds):
        """Initialize the watchdog with a timeout in seconds."""
        self.timeout = timeout_seconds
        self._timer = None

    def _handle_timeout(self):
        print("Watchdog timeout, terminating the application")
        # Immediately terminates the process with the given status code,
        # bypassing the usual cleanup process that Python performs when exiting.
        # This means that any cleanup handlers, such as try...finally blocks, with statements,
        # and atexit functions, will not be executed.
        # This ensures that the process terminates immediately.
        os._exit(1)  # pylint: disable=protected-access

    def start(self):
        """Start the watchdog timer."""
        self._timer = threading.Timer(self.timeout, self._handle_timeout)
        self._timer.start()

    def report_healthy(self):
        """Report that the application is healthy, resetting the watchdog timer."""
        if self._timer:
            self._timer.cancel()
        self.start()
