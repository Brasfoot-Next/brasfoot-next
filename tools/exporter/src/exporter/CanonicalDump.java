package exporter;

import com.esotericsoftware.kryo.Kryo;
import com.esotericsoftware.kryo.Serializer;
import com.esotericsoftware.kryo.io.Input;
import com.esotericsoftware.kryo.serializers.FieldSerializer;
import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;

/**
 * Loads a .s22 exactly like the game (c.a.c(String, boolean)) and writes the object graph in the
 * canonical text form that crates/persistence also produces from its own Kryo reader. The two
 * outputs must be byte-identical; this is the differential test of the Rust save reader.
 *
 * One value per line, depth first, fields in Kryo order (only the fields Kryo serializes):
 *   N                      null
 *   i/l/s/b/c <n>          int, long, short, byte, char (code unit); boxed or primitive alike
 *   z 0|1                  boolean
 *   d/f <hex bits>         double, float (raw IEEE bits)
 *   S <text>               string, escaped: \\ \n \r and \xHH below 0x20
 *   E <class> <ordinal>    enum constant
 *   ^<id>                  repeated object (ids count objects in first-visit order, from 1)
 *   {<id> <class> <n>      object with n serialized fields, each preceded by ".<name>"
 *   [<id> <class> <n>      array or collection with n items
 *   <<id> <class> <n>      map with n entries, key then value
 *   @<id> <class>          calendar, followed by: S zone, l millis, z lenient, i firstDayOfWeek,
 *                          i minimalDaysInFirstWeek, l gregorianChange
 *
 * Usage: CanonicalDump <file.s22> <out.txt>   (run with user.dir = a copy of the game folder)
 */
public class CanonicalDump {
   final Kryo kryo = new Kryo();
   final IdentityHashMap<Object, Integer> ids = new IdentityHashMap<>();
   final Map<Class<?>, Field[]> fieldCache = new HashMap<>();
   int nextId = 1;
   final Writer w;

   CanonicalDump(Writer w) {
      this.w = w;
      kryo.setRegistrationRequired(false);
   }

   Field[] fieldsOf(Class<?> c) throws Exception {
      Field[] r = fieldCache.get(c);
      if (r != null) return r;
      Serializer<?> s = kryo.getSerializer(c);
      if (!(s instanceof FieldSerializer<?> fs)) throw new IllegalStateException("not a FieldSerializer class: " + c.getName());
      Field fieldOf = FieldSerializer.CachedField.class.getDeclaredField("field");
      fieldOf.setAccessible(true);
      FieldSerializer.CachedField<?>[] cached = fs.getFields();
      r = new Field[cached.length];
      for (int i = 0; i < cached.length; i++) {
         r[i] = (Field) fieldOf.get(cached[i]);
         r[i].setAccessible(true);
      }
      fieldCache.put(c, r);
      return r;
   }

   void line(String s) throws IOException {
      w.write(s);
      w.write('\n');
   }

   void str(String s) throws IOException {
      StringBuilder b = new StringBuilder(s.length() + 2).append("S ");
      for (int i = 0; i < s.length(); i++) {
         char c = s.charAt(i);
         if (c == '\\') b.append("\\\\");
         else if (c == '\n') b.append("\\n");
         else if (c == '\r') b.append("\\r");
         else if (c < 0x20) b.append(String.format("\\x%02x", (int) c));
         else b.append(c);
      }
      line(b.toString());
   }

   void value(Object o) throws Exception {
      if (o == null) {
         line("N");
         return;
      }
      if (o instanceof String s) {
         str(s);
         return;
      }
      if (o instanceof Integer || o instanceof Long || o instanceof Short || o instanceof Byte) {
         line((o instanceof Integer ? "i " : o instanceof Long ? "l " : o instanceof Short ? "s " : "b ") + o);
         return;
      }
      if (o instanceof Character ch) {
         line("c " + (int) ch);
         return;
      }
      if (o instanceof Boolean z) {
         line(z ? "z 1" : "z 0");
         return;
      }
      if (o instanceof Double d) {
         line("d " + Long.toHexString(Double.doubleToRawLongBits(d)));
         return;
      }
      if (o instanceof Float f) {
         line("f " + Integer.toHexString(Float.floatToRawIntBits(f)));
         return;
      }
      if (o instanceof Enum<?> e) {
         line("E " + e.getDeclaringClass().getName() + " " + e.ordinal());
         return;
      }
      Integer seen = ids.get(o);
      if (seen != null) {
         line("^" + seen);
         return;
      }
      int id = nextId++;
      ids.put(o, id);
      Class<?> c = o.getClass();
      String name = c.getName();
      if (c.isArray()) {
         int n = Array.getLength(o);
         line("[" + id + " " + name + " " + n);
         for (int i = 0; i < n; i++) value(Array.get(o, i));
         return;
      }
      if (o instanceof Calendar cal) {
         line("@" + id + " " + name);
         str(cal.getTimeZone().getID());
         line("l " + cal.getTimeInMillis());
         line(cal.isLenient() ? "z 1" : "z 0");
         line("i " + cal.getFirstDayOfWeek());
         line("i " + cal.getMinimalDaysInFirstWeek());
         line("l " + (cal instanceof GregorianCalendar g ? g.getGregorianChange().getTime() : -12219292800000L));
         return;
      }
      if (o instanceof Collection<?> col) {
         if (!(o instanceof List) && !(o instanceof LinkedHashSet)) throw new IllegalStateException("unordered collection " + name);
         line("[" + id + " " + name + " " + col.size());
         for (Object x : col) value(x);
         return;
      }
      if (o instanceof Map<?, ?> m) {
         if (!(o instanceof LinkedHashMap)) throw new IllegalStateException("unordered map " + name);
         line("<" + id + " " + name + " " + m.size());
         for (Map.Entry<?, ?> e : m.entrySet()) {
            value(e.getKey());
            value(e.getValue());
         }
         return;
      }
      Field[] fields = fieldsOf(c);
      line("{" + id + " " + name + " " + fields.length);
      for (Field f : fields) {
         line("." + f.getName());
         value(f.get(o));
      }
   }

   public static void main(String[] args) throws Exception {
      Path s22 = Paths.get(args[0]);
      Path out = Paths.get(args[1]);
      Throwable[] error = new Throwable[1];
      Thread t = new Thread(null, () -> {
         try {
            new best.M(true);
            best.n.cU();
            Kryo k = new Kryo();
            k.setRegistrationRequired(false);
            Object sr, afq;
            try (Input in = new Input(new FileInputStream(s22.toFile()))) {
               sr = k.readClassAndObject(in);
               afq = k.readClassAndObject(in);
            }
            try (Writer w = new BufferedWriter(new OutputStreamWriter(Files.newOutputStream(out), StandardCharsets.UTF_8), 1 << 20)) {
               // the two roots are written by separate writeClassAndObject calls: Kryo resets between them
               CanonicalDump first = new CanonicalDump(w);
               first.value(sr);
               CanonicalDump second = new CanonicalDump(w);
               second.value(afq);
               System.out.println("objects: " + (first.nextId - 1) + " + " + (second.nextId - 1));
            }
         } catch (Throwable e) {
            error[0] = e;
         }
      }, "canonical-dump", 4L << 30);
      t.start();
      t.join();
      if (error[0] != null) {
         error[0].printStackTrace();
         System.exit(1);
      }
   }
}
