package exporter;

import com.esotericsoftware.kryo.Kryo;
import com.esotericsoftware.kryo.io.Output;
import com.esotericsoftware.kryo.serializers.FieldSerializer;
import java.io.*;
import java.lang.reflect.*;
import java.util.*;
import org.objenesis.ObjenesisStd;

/**
 * Writes a fictional .s22: the two roots the game saves (best.f and best.ay), built from the
 * game's own classes and filled with made-up values (seeded, so the output is stable). Every
 * serialized field of every class a save holds gets a value, with shared objects, cycles, nulls,
 * boxed values in lists, arrays, calendars and strings of every encoding Kryo uses. It is the
 * fixture of the Rust reader's differential test; no real career is involved.
 *
 * Lists hold the element class the game puts in them (tools/exporter/list-elements.tsv, written by
 * tools/modelgen from the game's own saves).
 *
 * Usage: FictionalFixture <out.s22> <list-elements.tsv> [seed]
 */
public class FictionalFixture {
   // The classes found in saves written by the game (best, f, components, est and their JDK parts).
   static final String[] SAVE_CLASSES = {
      "best.f", "best.ay", "best.A", "best.C", "best.F", "best.G", "best.H", "best.I", "best.J", "best.T", "best.Z", "best.a",
      "best.ah", "best.aj", "best.ak", "best.al", "best.am", "best.ao", "best.ap", "best.b", "best.c", "best.e", "best.i",
      "best.j", "best.k", "best.u", "best.v", "components.ag", "components.am", "components.as", "components.t",
      "est.InfoArquivoSalvoType", "f.A", "f.B", "f.D", "f.E", "f.G", "f.H", "f.I", "f.J", "f.K", "f.L", "f.M", "f.N", "f.a",
      "f.c", "f.d", "f.e", "f.f", "f.g", "f.h", "f.i", "f.j", "f.k", "f.l", "f.m", "f.n", "f.p", "f.q", "f.r", "f.s", "f.t",
      "f.u", "f.v", "f.w", "f.x", "f.y", "f.z",
      // in saves of countries with state championships and regional cups
      "best.w", "f.o", "f.F"
   };
   static final String[] NAMES = {
      "Esporte Clube Fictício", "Atlético Imaginário", "União dos Testes", "Fulano de Tal", "Beltrano Silva",
      "Ciclano Souza", "João Exemplo", "Zé Ninguém", "Estádio da Invenção", "Arena Hipotética", "Grêmio Inventado",
      "Ação Ñandú Çaçapava", "x", "", "Técnico de Teste",
      "Liga das Nações Que Não Existem e Que Têm Um Nome Bem Comprido Para Passar de 64 Letras"
   };

   final Random random;
   final Kryo kryo = new Kryo();
   final ObjenesisStd objenesis = new ObjenesisStd();
   final List<Class<?>> pool = new ArrayList<>();
   final Map<Class<?>, List<Object>> made = new HashMap<>();
   final List<String> strings = new ArrayList<>();
   final Deque<Object> toFill = new ArrayDeque<>();
   int budget = 2500;
   // "class.field" -> {element class, "nullable" | "nonnull"}
   final Map<String, String[]> listElements = new HashMap<>();

   FictionalFixture(long seed) throws Exception {
      random = new Random(seed);
      kryo.setRegistrationRequired(false);
      for (String name : SAVE_CLASSES) pool.add(Class.forName(name, false, getClass().getClassLoader()));
   }

   boolean chance(double p) {
      return random.nextDouble() < p;
   }

   Object instance(Class<?> c) {
      List<Object> existing = made.computeIfAbsent(c, k -> new ArrayList<>());
      if (!existing.isEmpty() && (budget <= 0 || chance(0.35))) return existing.get(random.nextInt(existing.size()));
      if (budget <= 0) return null;
      budget--;
      Object o = objenesis.newInstance(c);
      existing.add(o);
      toFill.add(o);
      return o;
   }

   /** A value of a game class assignable to {@code declared}, or null. */
   Object gameObject(Class<?> declared) {
      List<Class<?>> fits = new ArrayList<>();
      for (Class<?> c : pool) if (declared.isAssignableFrom(c) && !Modifier.isAbstract(c.getModifiers())) fits.add(c);
      if (fits.isEmpty()) return null;
      return instance(fits.get(random.nextInt(fits.size())));
   }

   String string() {
      if (!strings.isEmpty() && chance(0.3)) return strings.get(random.nextInt(strings.size())); // same instance: a Kryo reference
      String s = NAMES[random.nextInt(NAMES.length)];
      if (chance(0.3)) s = s + " " + random.nextInt(100);
      s = new String(s);
      strings.add(s);
      return s;
   }


   GregorianCalendar calendar() {
      return new GregorianCalendar(2020 + random.nextInt(30), random.nextInt(12), 1 + random.nextInt(28));
   }

   /** A list of the field's element class; empty when the game never fills it. */
   ArrayList<Object> list(String owner, String field) throws Exception {
      ArrayList<Object> l = new ArrayList<>();
      String[] element = listElements.get(owner + "." + field);
      if (element == null) return l;
      int n = random.nextInt(4);
      for (int i = 0; i < n; i++) {
         if (element[1].equals("nullable") && chance(0.2)) l.add(null);
         else if (element[0].equals("java.lang.Integer")) l.add(random.nextInt(300) - 20);
         else if (element[0].equals("java.lang.String")) l.add(string());
         else {
            Object o = gameObject(Class.forName(element[0], false, getClass().getClassLoader()));
            if (o != null) l.add(o);
         }
      }
      return l;
   }

   Object array(Class<?> type) {
      if (chance(0.1)) return null;
      Class<?> component = type.getComponentType();
      int n = random.nextInt(5);
      Object a = Array.newInstance(component, n);
      for (int i = 0; i < n; i++) {
         Object v;
         if (component == int.class) v = random.nextInt(200) - 50;
         else if (component == double.class) v = chance(0.1) ? Double.NaN : random.nextGaussian() * 100;
         else if (component == boolean.class) v = random.nextBoolean();
         else if (component == long.class) v = random.nextLong();
         else if (component == float.class) v = random.nextFloat();
         else if (component == String.class) v = chance(0.1) ? null : string();
         else if (component.isArray()) v = array(component);
         else if (component == java.awt.Color.class) v = new java.awt.Color(random.nextInt(0xffffff));
         else v = gameObject(component);
         Array.set(a, i, v);
      }
      return a;
   }

   Object value(Class<?> type, String owner, String field) throws Exception {
      if (type == int.class) return chance(0.2) ? random.nextInt() : random.nextInt(300) - 20;
      if (type == long.class) return chance(0.3) ? random.nextLong() : (long) random.nextInt(1_000_000_000);
      if (type == double.class) return chance(0.05) ? -0.0 : random.nextGaussian() * 1000;
      if (type == float.class) return random.nextFloat();
      if (type == boolean.class) return random.nextBoolean();
      if (type == short.class) return (short) random.nextInt();
      if (type == byte.class) return (byte) random.nextInt();
      if (type == char.class) return (char) (32 + random.nextInt(200));
      if (chance(0.08)) return null;
      if (type == String.class) return string();
      if (type == Integer.class) return random.nextInt(1000);
      if (type == Boolean.class) return random.nextBoolean();
      if (type == Long.class) return random.nextLong();
      if (type == Double.class) return random.nextDouble();
      if (type.isArray()) return array(type);
      if (Calendar.class.isAssignableFrom(type)) return calendar();
      if (type == java.awt.Color.class) return new java.awt.Color(random.nextInt(0xffffff));
      if (type.isAssignableFrom(ArrayList.class)) return list(owner, field);
      if (type.isEnum()) return type.getEnumConstants()[random.nextInt(type.getEnumConstants().length)];
      return gameObject(type);
   }

   void fill(Object o) throws Exception {
      FieldSerializer<?> serializer = (FieldSerializer<?>) kryo.getSerializer(o.getClass());
      Field fieldOf = FieldSerializer.CachedField.class.getDeclaredField("field");
      fieldOf.setAccessible(true);
      for (FieldSerializer.CachedField<?> cf : serializer.getFields()) {
         Field f = (Field) fieldOf.get(cf);
         f.setAccessible(true);
         Object v = value(f.getType(), o.getClass().getName(), f.getName());
         if (v != null || !f.getType().isPrimitive()) f.set(o, v);
      }
   }

   void write(File out) throws Exception {
      Object game = objenesis.newInstance(Class.forName("best.f"));
      Object second = objenesis.newInstance(Class.forName("best.ay"));
      made.computeIfAbsent(game.getClass(), k -> new ArrayList<>()).add(game);
      toFill.add(game);
      toFill.add(second);
      // make sure every class shows up at least once
      for (Class<?> c : pool) if (!Modifier.isAbstract(c.getModifiers()) && !made.containsKey(c)) instance(c);
      while (!toFill.isEmpty()) fill(toFill.poll());
      try (Output output = new Output(new FileOutputStream(out))) {
         kryo.writeClassAndObject(output, game);
         kryo.writeClassAndObject(output, second);
      }
   }

   public static void main(String[] args) throws Exception {
      long seed = args.length > 2 ? Long.parseLong(args[2]) : 20261020L;
      Throwable[] error = new Throwable[1];
      Thread t = new Thread(null, () -> {
         try {
            FictionalFixture fixture = new FictionalFixture(seed);
            for (String line : java.nio.file.Files.readAllLines(java.nio.file.Paths.get(args[1]))) {
               String[] parts = line.split("\t");
               if (parts.length == 3) fixture.listElements.put(parts[0], new String[]{parts[1], parts[2]});
            }
            fixture.write(new File(args[0]));
         } catch (Throwable e) {
            error[0] = e;
         }
      }, "fixture", 1L << 30);
      t.start();
      t.join();
      if (error[0] != null) {
         error[0].printStackTrace();
         System.exit(1);
      }
      System.out.println("wrote " + args[0] + " (" + new File(args[0]).length() + " bytes)");
   }
}
