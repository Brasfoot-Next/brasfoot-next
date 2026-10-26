package exporter;

import com.esotericsoftware.kryo.Kryo;
import com.esotericsoftware.kryo.io.Input;
import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;
import java.util.zip.GZIPOutputStream;

/**
 * Generic, reflection based dumper of Java object graphs to JSON.
 * No knowledge of the game's classes: every field of every reachable object is written.
 * See README.md for the format.
 */
public class Exporter {
   // ---- statistics ----
   static final class FieldStat {
      String name, type, declaring;
      LinkedHashSet<String> observed = new LinkedHashSet<>();
   }
   static final class ClassStat {
      long instances;
      final LinkedHashMap<String, FieldStat> fields = new LinkedHashMap<>();
      String superclass;
   }
   final Map<String, ClassStat> stats = new TreeMap<>();
   final Map<String, Long> skipped = new TreeMap<>();

   final IdentityHashMap<Object, Integer> ids = new IdentityHashMap<>();
   int nextId = 1;
   Writer w;

   final Map<Class<?>, FieldInfo[]> fieldCache = new HashMap<>();

   record FieldInfo(Field f, String key, Class<?> declaring) {}

   // ---- field discovery ----
   static boolean isJdk(Class<?> c) {
      String n = c.getName();
      return n.startsWith("java.") || n.startsWith("javax.") || n.startsWith("jdk.") || n.startsWith("sun.");
   }

   FieldInfo[] fieldsOf(Class<?> c) {
      FieldInfo[] r = fieldCache.get(c);
      if (r != null) return r;
      List<Class<?>> hier = new ArrayList<>();
      for (Class<?> k = c; k != null && k != Object.class; k = k.getSuperclass()) hier.add(0, k);
      Map<String, Integer> cont = new HashMap<>();
      List<Field> fs = new ArrayList<>();
      for (Class<?> k : hier) {
         for (Field f : k.getDeclaredFields()) {
            if (Modifier.isStatic(f.getModifiers())) continue;
            fs.add(f);
            cont.merge(f.getName(), 1, Integer::sum);
         }
      }
      List<FieldInfo> out = new ArrayList<>();
      for (Field f : fs) {
         boolean clash = cont.get(f.getName()) > 1;
         String key = clash ? f.getDeclaringClass().getName() + "#" + f.getName() : f.getName();
         try {
            f.setAccessible(true);
         } catch (RuntimeException e) {
            // inaccessible JDK internals: handled at read time
         }
         out.add(new FieldInfo(f, key, f.getDeclaringClass()));
      }
      r = out.toArray(new FieldInfo[0]);
      fieldCache.put(c, r);
      return r;
   }

   ClassStat stat(Class<?> c) {
      return stats.computeIfAbsent(className(c), k -> {
         ClassStat s = new ClassStat();
         s.superclass = c.getSuperclass() == null ? null : c.getSuperclass().getName();
         if (!c.isArray() && !isJdk(c)) {
            for (FieldInfo cp : fieldsOf(c)) {
               FieldStat cs = new FieldStat();
               cs.name = cp.key();
               cs.type = cp.f().getGenericType().getTypeName();
               cs.declaring = cp.declaring().getName();
               s.fields.put(cp.key(), cs);
            }
         }
         return s;
      });
   }

   static String className(Class<?> c) {
      return c.getName();
   }

   void observe(ClassStat s, String key, String obs) {
      FieldStat cs = s.fields.get(key);
      if (cs == null) return;
      if (cs.observed.size() < 8) cs.observed.add(obs);
      else if (cs.observed.size() == 8) cs.observed.add("...");
   }

   // ---- JSON primitives ----
   void str(String s) throws IOException {
      w.write('"');
      int n = s.length();
      for (int i = 0; i < n; i++) {
         char c = s.charAt(i);
         switch (c) {
            case '"' -> w.write("\\\"");
            case '\\' -> w.write("\\\\");
            case '\n' -> w.write("\\n");
            case '\r' -> w.write("\\r");
            case '\t' -> w.write("\\t");
            default -> {
               if (c < 0x20 || c == 0x2028 || c == 0x2029 || c == 0x7f) {
                  w.write(String.format("\\u%04x", (int) c));
               } else if (Character.isHighSurrogate(c) && i + 1 < n && Character.isLowSurrogate(s.charAt(i + 1))) {
                  w.write(c);
                  w.write(s.charAt(++i));
               } else if (Character.isSurrogate(c)) {
                  w.write(String.format("\\u%04x", (int) c));
               } else {
                  w.write(c);
               }
            }
         }
      }
      w.write('"');
   }

   static final long LIM = 1L << 53;

   void longo(long v) throws IOException {
      if (v > LIM || v < -LIM) str(Long.toString(v));
      else w.write(Long.toString(v));
   }

   void dbl(double d) throws IOException {
      if (Double.isNaN(d) || Double.isInfinite(d)) str(Double.toString(d));
      else w.write(Double.toString(d));
   }

   void flt(float d) throws IOException {
      if (Float.isNaN(d) || Float.isInfinite(d)) str(Float.toString(d));
      else w.write(Float.toString(d));
   }

   // ---- skip rules ----
   static boolean shouldSkip(Object o) {
      return o instanceof java.awt.Image || o instanceof javax.swing.Icon || o instanceof java.awt.Component
         || o instanceof java.awt.Graphics || o instanceof java.awt.Font || o instanceof java.awt.Toolkit
         || o instanceof Class || o instanceof Thread || o instanceof ClassLoader || o instanceof java.io.Closeable
         || o instanceof java.awt.image.ImageProducer || o instanceof java.awt.image.ImageObserver
         || o instanceof java.awt.Cursor || o instanceof javax.swing.border.Border;
   }

   // ---- main recursive writer ----
   /** Writes any value. 'decl' is the declared type (null when unknown), used to keep boxed values unambiguous. */
   void value(Object o, Class<?> decl) throws IOException {
      if (o == null) {
         w.write("null");
         return;
      }
      Class<?> c = o.getClass();
      if (o instanceof String s) {
         str(s);
         return;
      }
      if (o instanceof Integer i) {
         if (decl == int.class || decl == Integer.class || decl == null || decl == Object.class || decl == Number.class || true) w.write(i.toString());
         return;
      }
      if (o instanceof Boolean b) {
         w.write(b ? "true" : "false");
         return;
      }
      if (o instanceof Double d) {
         dbl(d);
         return;
      }
      if (o instanceof Long || o instanceof Short || o instanceof Byte || o instanceof Float || o instanceof Character) {
         // exact boxed type is kept unless the declared type already says it
         boolean typed = decl == c || (decl != null && decl.isPrimitive());
         if (!typed) {
            w.write("{\"$box\":");
            str(c.getName());
            w.write(",\"value\":");
         }
         if (o instanceof Long l) longo(l);
         else if (o instanceof Float f) flt(f);
         else if (o instanceof Character ch) str(String.valueOf(ch));
         else w.write(o.toString());
         if (!typed) w.write('}');
         return;
      }
      if (o instanceof Enum<?> e) {
         w.write("{\"$enum\":");
         str(e.getDeclaringClass().getName());
         w.write(",\"name\":");
         str(e.name());
         w.write(",\"ordinal\":" + e.ordinal() + "}");
         stat(e.getDeclaringClass()).instances++;
         return;
      }
      if (o instanceof Class<?> k) {
         w.write("{\"$classRef\":");
         str(k.getName());
         w.write("}");
         return;
      }
      if (shouldSkip(o)) {
         skipped.merge(c.getName(), 1L, Long::sum);
         w.write("{\"$class\":");
         str(c.getName());
         w.write(",\"$skipped\":true}");
         return;
      }
      Integer seen = ids.get(o);
      if (seen != null) {
         w.write("{\"$ref\":" + seen + "}");
         return;
      }
      int id = nextId++;
      ids.put(o, id);
      ClassStat st = stat(c);
      st.instances++;
      w.write("{\"$id\":" + id + ",\"$class\":");
      str(c.getName());

      if (c.isArray()) {
         w.write(",\"items\":[");
         int n = Array.getLength(o);
         Class<?> comp = c.getComponentType();
         for (int i = 0; i < n; i++) {
            if (i > 0) w.write(',');
            if (comp.isPrimitive()) prim(Array.get(o, i), comp);
            else value(Array.get(o, i), comp);
         }
         w.write("]}");
         return;
      }
      if (o instanceof Map<?, ?> m) {
         w.write(",\"entries\":[");
         boolean first = true;
         for (Map.Entry<?, ?> en : m.entrySet()) {
            if (!first) w.write(',');
            first = false;
            w.write('[');
            value(en.getKey(), null);
            w.write(',');
            value(en.getValue(), null);
            w.write(']');
         }
         w.write(']');
         extras(o, c, st);
         w.write('}');
         return;
      }
      if (o instanceof Collection<?> col) {
         w.write(",\"items\":[");
         boolean first = true;
         for (Object x : col) {
            if (!first) w.write(',');
            first = false;
            value(x, null);
         }
         w.write(']');
         extras(o, c, st);
         w.write('}');
         return;
      }
      if (o instanceof Calendar cal) {
         w.write(",\"millis\":");
         longo(cal.getTimeInMillis());
         w.write(",\"timeZone\":");
         str(cal.getTimeZone().getID());
         w.write(",\"lenient\":" + cal.isLenient() + ",\"firstDayOfWeek\":" + cal.getFirstDayOfWeek()
            + ",\"minimalDaysInFirstWeek\":" + cal.getMinimalDaysInFirstWeek() + "}");
         return;
      }
      if (o instanceof TimeZone tz) {
         w.write(",\"id\":");
         str(tz.getID());
         w.write('}');
         return;
      }
      if (o instanceof java.awt.Color col) {
         w.write(",\"argb\":" + col.getRGB() + "}");
         return;
      }
      if (o instanceof CharSequence cs) {
         w.write(",\"text\":");
         str(cs.toString());
         w.write('}');
         return;
      }
      // plain object: every field, including inherited and private ones
      w.write(",\"fields\":{");
      objectFields(o, c, st, true);
      w.write("}}");
   }

   /** Extra declared fields of game subclasses of collections/maps. */
   void extras(Object o, Class<?> c, ClassStat st) throws IOException {
      if (isJdk(c)) return;
      w.write(",\"fields\":{");
      objectFields(o, c, st, false);
      w.write('}');
   }

   void objectFields(Object o, Class<?> c, ClassStat st, boolean todosJdk) throws IOException {
      boolean first = true;
      for (FieldInfo cp : fieldsOf(c)) {
         if (!todosJdk && isJdk(cp.declaring())) continue;
         Field f = cp.f();
         if (!first) w.write(',');
         first = false;
         str(cp.key());
         w.write(':');
         Class<?> t = f.getType();
         Object v;
         try {
            if (t.isPrimitive()) {
               v = f.get(o);
               prim(v, t);
               continue;
            }
            v = f.get(o);
         } catch (IllegalAccessException | RuntimeException e) {
            skipped.merge("(inaccessible) " + cp.declaring().getName() + "." + f.getName(), 1L, Long::sum);
            w.write("{\"$unreadable\":true}");
            continue;
         }
         if (!st.fields.isEmpty()) observe(st, cp.key(), describe(v));
         value(v, t);
      }
   }

   void prim(Object v, Class<?> t) throws IOException {
      if (t == long.class) longo((Long) v);
      else if (t == double.class) dbl((Double) v);
      else if (t == float.class) flt((Float) v);
      else if (t == char.class) str(String.valueOf((char) (Character) v));
      else if (t == boolean.class) w.write((Boolean) v ? "true" : "false");
      else w.write(v.toString());
   }

   static String describe(Object v) {
      if (v == null) return "null";
      Class<?> c = v.getClass();
      if (c.isArray()) {
         return c.getComponentType().getName() + "[] -> " + elements(Arrays.asList(v instanceof Object[] a ? a : new Object[0]), c);
      }
      if (v instanceof Collection<?> col) return c.getName() + "<" + elements(col, c) + ">";
      if (v instanceof Map<?, ?> m) {
         return c.getName() + "<" + elements(m.keySet(), c) + ", " + elements(m.values(), c) + ">";
      }
      return c.getName();
   }

   static String elements(Collection<?> col, Class<?> c) {
      LinkedHashSet<String> s = new LinkedHashSet<>();
      int n = 0;
      for (Object x : col) {
         if (n++ > 200) break;
         s.add(x == null ? "null" : x.getClass().getName());
         if (s.size() > 5) {
            s.add("...");
            break;
         }
      }
      return String.join("|", s);
   }

   // ---- inputs ----
   Object readKryo(Input in) {
      Kryo k = new Kryo();
      k.setRegistrationRequired(false);
      return k.readClassAndObject(in);
   }

   static Object readSerialized(Path p) throws Exception {
      try (ObjectInputStream in = new ObjectInputStream(new BufferedInputStream(Files.newInputStream(p)))) {
         return in.readObject();
      }
   }

   void open(Path saida, boolean gzip) throws IOException {
      OutputStream os = new BufferedOutputStream(Files.newOutputStream(saida), 1 << 20);
      if (gzip) os = new GZIPOutputStream(os, 1 << 16);
      w = new BufferedWriter(new OutputStreamWriter(os, StandardCharsets.UTF_8), 1 << 16);
   }

   void summary(Path p, String input) throws IOException {
      try (PrintWriter pw = new PrintWriter(Files.newBufferedWriter(p))) {
         pw.println("# Export summary: " + input);
         pw.println("# objects with id: " + (nextId - 1) + " ; classes: " + stats.size());
         pw.println("# per class: name | instances | superclass ; then one field per line:");
         pw.println("#   field-name : declared-type   [declaring class if inherited]   -> observed runtime types (up to 8)");
         pw.println("# Fields whose name repeats in the hierarchy appear as DeclaringClass#name.");
         pw.println("# java.* classes are listed without fields (collections become items/entries).");
         pw.println();
         stats.entrySet().stream().sorted((a, b) -> Long.compare(b.getValue().instances, a.getValue().instances))
            .forEach(e -> {
               ClassStat s = e.getValue();
               pw.println(e.getKey() + " | " + s.instances + " | extends " + s.superclass);
               for (FieldStat c : s.fields.values()) {
                  StringBuilder sb = new StringBuilder("    " + c.name + " : " + c.type);
                  if (!c.name.contains("#") && !c.declaring.equals(e.getKey())) sb.append("   [").append(c.declaring).append("]");
                  if (!c.observed.isEmpty() && !isPlain(c.type)) sb.append("   -> ").append(String.join(" ; ", c.observed));
                  pw.println(sb);
               }
            });
         pw.println();
         pw.println("# Skipped / unreadable (class: occurrences)");
         if (skipped.isEmpty()) pw.println("(none)");
         skipped.forEach((k, v) -> pw.println(k + ": " + v));
      }
   }

   static boolean isPlain(String t) {
      return switch (t) {
         case "int", "long", "short", "byte", "char", "boolean", "float", "double", "java.lang.String" -> true;
         default -> false;
      };
   }

   void close() throws IOException {
      w.flush();
      w.close();
   }

   // ---- commands ----
   static void runBig(Runnable r) throws Exception {
      Throwable[] err = new Throwable[1];
      Thread t = new Thread(null, () -> {
         try {
            r.run();
         } catch (Throwable e) {
            err[0] = e;
         }
      }, "exportador", 4L << 30);
      t.start();
      t.join();
      if (err[0] != null) {
         err[0].printStackTrace();
         System.exit(1);
      }
   }

   public static void main(String[] args) throws Exception {
      boolean gzip = false;
      List<String> a = new ArrayList<>();
      for (String s : args) {
         if (s.equals("--gzip")) gzip = true;
         else a.add(s);
      }
      final boolean gz = gzip;
      String cmd = a.get(0);
      String outDir = a.get(1);
      Files.createDirectories(Paths.get(outDir));
      String ext = gz ? ".json.gz" : ".json";
      switch (cmd) {
         case "save" -> {
            // save <outDir> <path.s22>
            Path s22 = Paths.get(a.get(2));
            String name = s22.getFileName().toString().replaceAll("\\.s22$", "");
            runBig(() -> {
               try {
                  new best.M(true);
                  best.n.cU();
                  Exporter ex = new Exporter();
                  Kryo k = new Kryo();
                  k.setRegistrationRequired(false);
                  Object sr, afq;
                  try (Input in = new Input(new FileInputStream(s22.toFile()))) {
                     sr = k.readClassAndObject(in);
                     afq = k.readClassAndObject(in);
                  }
                  Object[] rest = new Object[0];
                  ex.open(Paths.get(outDir, name + ext), gz);
                  ex.w.write("{\"input\":");
                  ex.str(s22.getFileName().toString());
                  ex.w.write(",\"format\":\"kryo: readClassAndObject twice -> c.a.SR (best.f), c.a.afQ (d.ay)\",\"roots\":{\"SR\":");
                  ex.value(sr, null);
                  ex.w.write(",\"afQ\":");
                  ex.value(afq, null);
                  ex.w.write("}}\n");
                  ex.close();
                  // only after the dump, so the exported value of bj is the original one
                  Field bj = best.f.class.getDeclaredField("bj");
                  bj.setAccessible(true);
                  bj.set(sr, "zz_teste_mod");
                  ex.summary(Paths.get(outDir, name + ".summary.txt"), s22.toString());
                  System.out.println("objects: " + (ex.nextId - 1) + " classes: " + ex.stats.size());
               } catch (Exception e) {
                  throw new RuntimeException(e);
               }
            });
         }
         case "ser" -> {
            // ser <outDir> <nameSaida> <files or dirs...>: java serialization; one JSON with all files
            String name = a.get(2);
            List<Path> files = new ArrayList<>();
            for (String s : a.subList(3, a.size())) {
               Path p = Paths.get(s);
               if (Files.isDirectory(p)) {
                  try (var st = Files.list(p)) {
                     st.filter(Files::isRegularFile).filter(x -> !x.getFileName().toString().startsWith(".")).sorted().forEach(files::add);
                  }
               } else files.add(p);
            }
            runBig(() -> {
               try {
                  new best.M(true);
                  best.n.cU();
                  Exporter ex = new Exporter();
                  ex.open(Paths.get(outDir, name + ext), gz);
                  ex.w.write("{\"format\":\"java serialization (ObjectInputStream), one root per file\",\"files\":[\n");
                  boolean first = true;
                  int failures = 0;
                  for (Path p : files) {
                     Object o;
                     try {
                        o = readSerialized(p);
                     } catch (Throwable e) {
                        System.err.println("failed " + p + ": " + e);
                        failures++;
                        continue;
                     }
                     if (!first) ex.w.write(",\n");
                     first = false;
                     ex.w.write("{\"file\":");
                     ex.str(p.getFileName().toString());
                     ex.w.write(",\"root\":");
                     ex.value(o, null);
                     ex.w.write('}');
                  }
                  ex.w.write("\n]}\n");
                  ex.close();
                  ex.summary(Paths.get(outDir, name + ".summary.txt"), name + " (" + files.size() + " files, " + failures + " failures)");
                  System.out.println("files: " + files.size() + " failures: " + failures + " objects: " + (ex.nextId - 1) + " classes: " + ex.stats.size());
               } catch (Exception e) {
                  throw new RuntimeException(e);
               }
            });
         }
         default -> throw new IllegalArgumentException(cmd);
      }
   }
}
