package exporter;

import com.esotericsoftware.kryo.Kryo;
import com.esotericsoftware.kryo.Serializer;
import com.esotericsoftware.kryo.serializers.FieldSerializer;
import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;

/**
 * Writes the Kryo 4 wire schema of every game class: which serializer a default {@code new Kryo()}
 * picks for it and, for FieldSerializer classes, the serialized fields in Kryo's order.
 * The Rust save reader (crates/persistence/src/kryo) is driven by this file.
 *
 * Usage: KryoSchema <out.json> <class-dir-or-jar-dir>... [--extra name,name,...]
 */
public class KryoSchema {
   static final String[] GAME_PACKAGES = {"a", "b", "best", "c", "components", "d", "e", "est", "f", "forms", "g", "h"};
   // JDK classes that saves can hold, besides the ones reached through declared field types.
   static final String[] JDK_CLASSES = {
      "java.lang.String", "java.lang.Integer", "java.lang.Long", "java.lang.Double", "java.lang.Float", "java.lang.Boolean",
      "java.lang.Short", "java.lang.Byte", "java.lang.Character", "java.util.ArrayList", "java.util.LinkedList", "java.util.Vector",
      "java.util.HashMap", "java.util.LinkedHashMap", "java.util.Hashtable", "java.util.HashSet", "java.util.LinkedHashSet",
      "java.util.GregorianCalendar", "java.awt.Color", "[I", "[[I", "[D", "[[D", "[Z", "[J", "[F", "[S", "[B", "[C",
      "[Ljava.lang.String;", "[Ljava.awt.Color;"
   };

   final Kryo kryo = new Kryo();
   final Map<String, Map<String, Object>> classes = new TreeMap<>();
   final Deque<Class<?>> pending = new ArrayDeque<>();
   final Set<String> seen = new HashSet<>();

   KryoSchema() {
      kryo.setRegistrationRequired(false);
   }

   void enqueue(Class<?> c) {
      if (c == null || c.isPrimitive() || c.isInterface() || c == Object.class) return;
      if (c.isArray()) {
         enqueue(c.getComponentType());
      }
      if (seen.add(c.getName())) pending.add(c);
   }

   void run() {
      while (!pending.isEmpty()) {
         Class<?> c = pending.poll();
         try {
            classes.put(c.getName(), describe(c));
         } catch (Throwable e) {
            Map<String, Object> m = new LinkedHashMap<>();
            m.put("serializer", "error");
            m.put("error", e.toString());
            classes.put(c.getName(), m);
         }
      }
   }

   Map<String, Object> describe(Class<?> c) throws Exception {
      Map<String, Object> m = new LinkedHashMap<>();
      m.put("final", kryo.isFinal(c));
      if (Modifier.isAbstract(c.getModifiers()) && !c.isArray()) {
         m.put("serializer", "abstract");
         return m;
      }
      Serializer<?> s = kryo.getSerializer(c);
      String sc = s.getClass().getName();
      String simple = sc.substring(sc.lastIndexOf('.') + 1).replace('$', '.');
      m.put("serializer", kind(simple));
      if (c.isArray()) {
         m.put("component", c.getComponentType().getName());
      }
      if (c.isEnum()) {
         List<String> names = new ArrayList<>();
         for (Object o : c.getEnumConstants()) names.add(((Enum<?>) o).name());
         m.put("constants", names);
      }
      if (s instanceof FieldSerializer<?> fs) {
         m.put("fields", fields(fs.getFields()));
         m.put("transient", fields(fs.getTransientFields()));
      }
      return m;
   }

   static String kind(String serializer) {
      return switch (serializer) {
         case "FieldSerializer" -> "field";
         case "CollectionSerializer" -> "collection";
         case "MapSerializer" -> "map";
         case "DefaultSerializers.StringSerializer" -> "string";
         case "DefaultSerializers.IntSerializer" -> "int";
         case "DefaultSerializers.LongSerializer" -> "long";
         case "DefaultSerializers.DoubleSerializer" -> "double";
         case "DefaultSerializers.FloatSerializer" -> "float";
         case "DefaultSerializers.BooleanSerializer" -> "boolean";
         case "DefaultSerializers.ShortSerializer" -> "short";
         case "DefaultSerializers.ByteSerializer" -> "byte";
         case "DefaultSerializers.CharSerializer" -> "char";
         case "DefaultSerializers.CalendarSerializer" -> "calendar";
         case "DefaultSerializers.EnumSerializer" -> "enum";
         case "DefaultArraySerializers.IntArraySerializer" -> "int-array";
         case "DefaultArraySerializers.LongArraySerializer" -> "long-array";
         case "DefaultArraySerializers.DoubleArraySerializer" -> "double-array";
         case "DefaultArraySerializers.FloatArraySerializer" -> "float-array";
         case "DefaultArraySerializers.BooleanArraySerializer" -> "boolean-array";
         case "DefaultArraySerializers.ShortArraySerializer" -> "short-array";
         case "DefaultArraySerializers.ByteArraySerializer" -> "byte-array";
         case "DefaultArraySerializers.CharArraySerializer" -> "char-array";
         case "DefaultArraySerializers.StringArraySerializer" -> "string-array";
         case "DefaultArraySerializers.ObjectArraySerializer" -> "object-array";
         default -> "unsupported:" + serializer;
      };
   }

   List<Map<String, Object>> fields(FieldSerializer.CachedField<?>[] cached) throws Exception {
      Field fieldOf = FieldSerializer.CachedField.class.getDeclaredField("field");
      Field valueClassOf = FieldSerializer.CachedField.class.getDeclaredField("valueClass");
      Field canBeNullOf = FieldSerializer.CachedField.class.getDeclaredField("canBeNull");
      fieldOf.setAccessible(true);
      valueClassOf.setAccessible(true);
      canBeNullOf.setAccessible(true);
      List<Map<String, Object>> out = new ArrayList<>();
      for (FieldSerializer.CachedField<?> cf : cached) {
         Field f = (Field) fieldOf.get(cf);
         Class<?> valueClass = (Class<?>) valueClassOf.get(cf);
         Map<String, Object> m = new LinkedHashMap<>();
         m.put("name", f.getName());
         m.put("declaring", f.getDeclaringClass().getName());
         Class<?> t = f.getType();
         if (t.isPrimitive()) {
            m.put("type", t.getName());
         } else {
            m.put("type", "object");
            m.put("declared", t.getName());
            if (valueClass != null) m.put("valueClass", valueClass.getName());
            m.put("canBeNull", canBeNullOf.get(cf));
            enqueue(t);
         }
         out.add(m);
      }
      return out;
   }

   // ---- JSON writing (no dependency available on the game's classpath) ----
   static void json(Writer w, Object o, String indent) throws IOException {
      if (o == null) w.write("null");
      else if (o instanceof String s) str(w, s);
      else if (o instanceof Boolean || o instanceof Number) w.write(o.toString());
      else if (o instanceof Map<?, ?> m) {
         boolean small = m.values().stream().noneMatch(v -> v instanceof Map || v instanceof List);
         if (small) {
            w.write('{');
            boolean first = true;
            for (Map.Entry<?, ?> e : m.entrySet()) {
               if (!first) w.write(", ");
               first = false;
               str(w, (String) e.getKey());
               w.write(": ");
               json(w, e.getValue(), indent);
            }
            w.write('}');
            return;
         }
         w.write("{\n");
         boolean first = true;
         for (Map.Entry<?, ?> e : m.entrySet()) {
            if (!first) w.write(",\n");
            first = false;
            w.write(indent + "  ");
            str(w, (String) e.getKey());
            w.write(": ");
            json(w, e.getValue(), indent + "  ");
         }
         w.write("\n" + indent + "}");
      } else if (o instanceof List<?> l) {
         if (l.isEmpty()) {
            w.write("[]");
            return;
         }
         boolean strings = l.get(0) instanceof String;
         w.write(strings ? "[" : "[\n");
         boolean first = true;
         for (Object x : l) {
            if (!first) w.write(strings ? ", " : ",\n");
            first = false;
            if (!strings) w.write(indent + "  ");
            json(w, x, indent + "  ");
         }
         w.write(strings ? "]" : "\n" + indent + "]");
      } else throw new IllegalArgumentException(o.getClass().getName());
   }

   static void str(Writer w, String s) throws IOException {
      w.write('"');
      for (char c : s.toCharArray()) {
         if (c == '"' || c == '\\') w.write("\\" + c);
         else if (c < 0x20) w.write(String.format("\\u%04x", (int) c));
         else w.write(c);
      }
      w.write('"');
   }

   public static void main(String[] args) throws Exception {
      Path out = Paths.get(args[0]);
      KryoSchema schema = new KryoSchema();
      ClassLoader loader = KryoSchema.class.getClassLoader();
      for (String name : JDK_CLASSES) schema.enqueue(Class.forName(name, false, loader));
      for (int i = 1; i < args.length; i++) {
         Path root = Paths.get(args[i]);
         for (String pkg : GAME_PACKAGES) {
            Path dir = root.resolve(pkg);
            if (!Files.isDirectory(dir)) continue;
            try (var files = Files.walk(dir)) {
               for (Path p : (Iterable<Path>) files::iterator) {
                  String rel = root.relativize(p).toString();
                  if (!rel.endsWith(".class")) continue;
                  String name = rel.substring(0, rel.length() - 6).replace(File.separatorChar, '.');
                  try {
                     Class<?> c = Class.forName(name, false, loader);
                     // anonymous and local classes never reach a save; Kryo cannot instantiate them anyway
                     if (c.isAnonymousClass() || c.isLocalClass() || c.isSynthetic()) continue;
                     schema.enqueue(c);
                  } catch (Throwable e) {
                     System.err.println("skipped " + name + ": " + e);
                  }
               }
            }
         }
      }
      schema.run();
      Map<String, Object> root = new LinkedHashMap<>();
      root.put("format", "kryo-4.0.2 default Kryo(), registrationRequired=false, references on, optimizedGenerics off");
      root.put("classes", schema.classes);
      try (Writer w = Files.newBufferedWriter(out, StandardCharsets.UTF_8)) {
         json(w, root, "");
         w.write('\n');
      }
      long errors = schema.classes.values().stream().filter(m -> "error".equals(m.get("serializer"))).count();
      System.out.println("classes: " + schema.classes.size() + " errors: " + errors);
   }
}
