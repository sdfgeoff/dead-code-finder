import calendar
import datetime


def future_date_as_unix_timestamp(
    delta: datetime.timedelta, start_time: datetime.datetime | None = None
) -> int:
    start_time = start_time or datetime.datetime.now(datetime.timezone.utc)
    end_time = start_time + delta
    return int(calendar.timegm(end_time.timetuple()))


future_date_as_unix_timestamp(datetime.timedelta(days=90))
