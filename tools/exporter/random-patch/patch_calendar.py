#!/usr/bin/env python3
"""Writes a java.util.GregorianCalendar whose "now" can be fixed for the differential tests
(`GregorianCalendar.bfSetNow(long)`; Long.MIN_VALUE goes back to the clock), and a
sun.util.locale.provider.CalendarProviderImpl (what Calendar.getInstance() uses) that asks it.
Usage: patch_calendar.py <GregorianCalendar.java> <out GregorianCalendar.java> <CalendarProviderImpl.java> <out CalendarProviderImpl.java>"""
import sys

provider = open(sys.argv[3]).read()
assert provider.count(".setInstant(System.currentTimeMillis())") == 1
open(sys.argv[4], "w").write(provider.replace(".setInstant(System.currentTimeMillis())", ".setInstant(java.util.GregorianCalendar.bfNowForProvider())"))

src = open(sys.argv[1]).read()
old = "        setTimeInMillis(System.currentTimeMillis());"
assert src.count(old) == 1, "clock read not found"
src = src.replace(old, "        setTimeInMillis(bfNow());")
marker = "    // Proleptic Gregorian dates"
anchor = src.index("public class GregorianCalendar extends Calendar {") + len("public class GregorianCalendar extends Calendar {")
addition = """

    // Brasfoot Next differential tests: a fixed "now" for new calendars when set.
    private static long bfNowOverride = Long.MIN_VALUE;

    private static synchronized long bfNow() {
        return bfNowOverride == Long.MIN_VALUE ? System.currentTimeMillis() : bfNowOverride;
    }

    public static synchronized void bfSetNow(long now) {
        bfNowOverride = now;
    }

    public static long bfNowForProvider() {
        return bfNow();
    }
"""
src = src[:anchor] + addition + src[anchor:]
open(sys.argv[2], "w").write(src)
