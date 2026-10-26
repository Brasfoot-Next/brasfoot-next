#!/usr/bin/env python3
"""Writes a java.util.Random whose no-arg constructor can take seeds from a counter, for the
differential tests (`Random.bfSetNext(long)` turns it on; Long.MIN_VALUE turns it off).
Usage: patch_random.py <Random.java from the JDK's src.zip> <out Random.java>"""
import sys

src = open(sys.argv[1]).read()
old = """    public Random() {
        this(seedUniquifier() ^ System.nanoTime());
    }"""
new = """    public Random() {
        this(bfNoArgSeed());
    }

    // Brasfoot Next differential tests: with a counter set, every no-arg Random takes the next
    // number as its seed, so the Java and Rust sides draw the same numbers.
    private static long bfNext = Long.MIN_VALUE;

    private static synchronized long bfNoArgSeed() {
        if (bfNext == Long.MIN_VALUE) {
            return seedUniquifier() ^ System.nanoTime();
        }
        if (bfTrace) {
            new Throwable("new Random() takes seed " + bfNext).printStackTrace(System.out);
        }
        return bfNext++;
    }

    // with the counter on, print where each no-arg Random comes from
    public static volatile boolean bfTrace = false;

    public static synchronized void bfSetNext(long next) {
        bfNext = next;
    }"""
assert old in src, "Random() constructor not found"
src = src.replace(old, new)
# every draw of every Random counted, for the traces of the differential tests (bfDraws)
old_next = """    protected int next(int bits) {
        long oldseed, nextseed;"""
new_next = """    public static long bfDraws = 0;

    protected int next(int bits) {
        bfDraws++;
        long oldseed, nextseed;"""
assert old_next in src, "next(int) not found"
open(sys.argv[2], "w").write(src.replace(old_next, new_next))
