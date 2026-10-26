package exporter;

import best.F;
import best.I;
import best.ah;
import best.al;
import com.esotericsoftware.kryo.Kryo;
import com.esotericsoftware.kryo.io.Input;
import com.esotericsoftware.kryo.io.Output;
import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.util.*;

/**
 * Differential reference for the Rust match engine (crates/engine). Builds a fictional world with
 * the game's own classes (made-up clubs, players and coaches), saves it like the game, loads it
 * back like the game, then plays AI matches with every no-arg Random drawing from a counter
 * (java.util.Random patched by export.sh) and writes what happened in a text form the Rust test
 * writes too.
 *
 * Usage (user.dir = a copy of the game folder, mod classes first on the classpath, the patched
 * Random in --patch-module java.base):
 *   EngineHarness <out-dir> <tactics:1|0> <first seed>
 * Writes <out-dir>/engine-world.s22 and <out-dir>/engine-<tactics>.txt.
 */
public class EngineHarness {
   static final String[] FIRST = {"Fulano", "Beltrano", "Ciclano", "João", "Zé", "Tício", "Mévio", "Caio", "Lúcio", "Tibúrcio"};
   static final String[] LAST = {"de Tal", "Exemplo", "Imaginário", "Fictício", "da Silva Teste", "Inventado", "Hipotético"};
   static final int CLUBS = 8;
   static final int SQUAD = 26;
   /** {competition kind, subkind}: national league, cup, state, continental, continental 2, national teams, friendly. */
   static final int[][] COMPETITIONS = {{1, 0}, {2, 0}, {3, 0}, {4, 0}, {6, 1}, {7, 3}, {0, 0}, {9, 2}};

   final Random setup = new Random(20261020L);

   static Object get(Object o, String field) throws Exception {
      Field f = find(o.getClass(), field);
      return f.get(o);
   }

   static void set(Object o, String field, Object value) throws Exception {
      find(o.getClass(), field).set(o, value);
   }

   static Field find(Class<?> c, String name) throws Exception {
      for (Class<?> k = c; k != null; k = k.getSuperclass()) {
         try {
            Field f = k.getDeclaredField(name);
            f.setAccessible(true);
            return f;
         } catch (NoSuchFieldException e) {
            // try the superclass
         }
      }
      throw new NoSuchFieldException(c.getName() + "." + name);
   }

   static void setStatic(Class<?> c, String name, Object value) throws Exception {
      Field f = c.getDeclaredField(name);
      sun.misc.Unsafe unsafe = unsafe();
      Object base = unsafe.staticFieldBase(f);
      long offset = unsafe.staticFieldOffset(f);
      unsafe.putObject(base, offset, value);
   }

   static sun.misc.Unsafe unsafe() throws Exception {
      Field f = sun.misc.Unsafe.class.getDeclaredField("theUnsafe");
      f.setAccessible(true);
      return (sun.misc.Unsafe) f.get(null);
   }

   String name() {
      return FIRST[setup.nextInt(FIRST.length)] + " " + LAST[setup.nextInt(LAST.length)];
   }

   /** The fictional world: clubs with squads, coaches, stadiums, competitions and one day of matches. */
   List<I> build() throws Exception {
      best.f game = new best.f();
      c.a.SR = game;
      c.a.afQ = new best.ay();
      game.k(1);
      game.l(0);
      game.setHabilidadeIndividual(true);
      best.a day = new best.a();
      day.a().set(2031, Calendar.MARCH, 15, 0, 0, 0);
      day.a().set(Calendar.MILLISECOND, 0);
      game.R().add(day);
      List<ah> clubs = new ArrayList<>();
      int playerNumber = 0;
      for (int i = 0; i < CLUBS; i++) {
         ah club = new ah();
         club.bX(100 + i);
         set(club, "dm", "Clube Fictício " + (i + 1));
         club.setPais(i < 6 ? 29 : 72);
         club.setDivisao(1 + i % 4);
         club.setReputacao(1 + i % 5);
         club.setNivel(10 + i);
         set(club, "mX", true);
         club.a("Estádio Imaginário " + (i + 1), 15000 + 4000 * i);
         al coach = new al("Técnico Hipotético " + (i + 1));
         game.a(coach);
         club.h(coach);
         coach.z(club);
         game.c(club);
         for (int j = 0; j < SQUAD; j++) {
            F p = new F();
            playerNumber++;
            set(p, "dm", name() + " " + playerNumber);
            p.setPais(club.getPais());
            int position = j < 3 ? 0 : j < 8 ? 1 : j < 13 ? 2 : j < 20 ? 3 : 4;
            p.setPosicao(position);
            p.setLado(setup.nextInt(2));
            int[] traits = F.at(position);
            p.setCr1(traits[0]);
            p.setCr2(traits[1]);
            p.setIdade(18 + setup.nextInt(17));
            int strength = 35 + setup.nextInt(55);
            p.ad(strength);
            p.aJ(position == 0 ? strength + setup.nextInt(10) : 1 + setup.nextInt(10));
            p.aI(20 + setup.nextInt(70));
            p.aK(20 + setup.nextInt(70));
            p.aL(20 + setup.nextInt(70));
            p.aM(position <= 2 ? strength : 10 + setup.nextInt(60));
            p.aN(20 + setup.nextInt(70));
            p.aO(position == 4 ? strength : 5 + setup.nextInt(60));
            p.aq(1 + setup.nextInt(10));
            p.ai(60 + setup.nextInt(41));
            p.fG();
            p.n(club);
            p.a(365L + setup.nextInt(700), true);
            club.kc().add(p);
            game.b(p);
         }
         // some designated takers, some of them gone from the squad
         club.k((F) club.kc().get(setup.nextInt(SQUAD)));
         club.n((F) club.kc().get(setup.nextInt(SQUAD)));
         clubs.add(club);
      }
      List<best.at> competitions = new ArrayList<>();
      for (int i = 0; i < COMPETITIONS.length; i++) {
         f.a c = new f.a();
         c.setNome("Competição Inventada " + (i + 1));
         set(c, "tR", COMPETITIONS[i][0]);
         set(c, "dz", COMPETITIONS[i][1]);
         competitions.add(c);
      }
      // a few players start suspended or injured
      for (int i = 0; i < 12; i++) {
         ah club = clubs.get(setup.nextInt(CLUBS));
         F p = (F) club.kc().get(setup.nextInt(SQUAD));
         best.at c = competitions.get(setup.nextInt(competitions.size()));
         best.H h = p.h(c);
         if (setup.nextBoolean()) h.cA(); else { h.cz(); h.cz(); h.cz(); }
      }
      for (int i = 0; i < 6; i++) {
         F p = (F) clubs.get(setup.nextInt(CLUBS)).kc().get(setup.nextInt(SQUAD));
         p.d(day.a().getTimeInMillis() + 86400000L * (1 + setup.nextInt(20)));
      }
      List<I> matches = new ArrayList<>();
      for (int round = 0; round < COMPETITIONS.length; round++) {
         for (int k = 0; k < CLUBS / 2; k++) {
            ah home = clubs.get((k + round) % CLUBS);
            ah away = clubs.get((CLUBS - 1 - k + round) % CLUBS);
            I m = new I();
            m.s(home);
            m.t(away);
            set(m, "fx", competitions.get(round));
            set(m, "fy", 0);
            set(m, "dH", home.ev());
            m.O(home.ev().dX());
            int[][] gn = (int[][]) get(m, "gn");
            int[][] gm = (int[][]) get(m, "gm");
            for (int s = 0; s < 2; s++) {
               for (int x = 0; x < 3; x++) gn[s][x] = x == 2 && setup.nextInt(3) == 0 ? -1 : 19 + setup.nextInt(20);
               for (int x = 0; x < 4; x++) gm[s][x] = x >= 2 && setup.nextInt(3) == 0 ? -1 : 5 + setup.nextInt(40);
            }
            day.a(m);
            day.a(competitions.get(round));
            matches.add(m);
         }
      }
      return matches;
   }

   /** Saves like c.a's save: the numbering of g.b.Ag and Ai first. */
   static void write(String path) throws Exception {
      g.b.Ag();
      g.b.Ai();
      Kryo k = new Kryo();
      k.setRegistrationRequired(false);
      try (Output out = new Output(new FileOutputStream(path))) {
         k.writeClassAndObject(out, c.a.SR);
         k.writeClassAndObject(out, c.a.afQ);
      }
   }

   /** Loads like c.a.c(String, boolean) and runs the load fix-ups of g.b. */
   static void load(String path) throws Exception {
      Kryo k = new Kryo();
      k.setRegistrationRequired(false);
      try (Input in = new Input(new FileInputStream(path))) {
         c.a.SR = (best.f) k.readClassAndObject(in);
         c.a.afQ = (best.ay) k.readClassAndObject(in);
      }
      g.b.Ah();
      g.b.Af();
      g.b.Aj();
      // c.a.c(String, boolean) also lists the clubs' stadiums by size (f.cb(), transient), which
      // the national teams' grounds come from (Z.C)
      c.a.SR.cb();
   }

   /** Classes whose static initialiser creates a Random: initialised before the counter starts. */
   static final String[] STATIC_RANDOMS = {
      "best.aq", "bfmod.Taticas", "bfmod.Motor", "bfmod.Satelites", "bfmod.Patrimonio", "bfmod.Comissao", "bfmod.Base",
      "bfmod.Cotas", "bfmod.Janelas", "bfmod.Economia", "bfmod.Emprestimos", "bfmod.Investidor", "bfmod.Mercado",
      "bfmod.Estadios", "bfmod.SuperMundial", "bfmod.Patrocinio", "bfmod.CampeonatoBase", "bfmod.Credito", "bfmod.Config"
   };

   /**
    * The mod classes with a static Random SORTE besides Taticas, in order: each gets the fixed seed
    * 900000 + its index, outside the counter (brasfoot_systems::STATIC_RANDOMS on the Rust side).
    */
   static final String[] MOD_RANDOMS = {
      "Comissao", "Base", "Estadios", "Emprestimos", "Cotas", "Credito", "Janelas", "CampeonatoBase", "Economia", "Investidor", "Patrocinio",
      "Mercado", "Patrimonio", "SuperMundial", "Satelites", "Motor"
   };

   /** Every static Random the matches use, re-created from the counter in a fixed order. */
   static void resetRandomness(long first) throws Exception {
      for (String c : STATIC_RANDOMS) Class.forName(c, true, EngineHarness.class.getClassLoader());
      Random.class.getMethod("bfSetNext", long.class).invoke(null, first);
      best.aq.tI = new Random();
      setStatic(Class.forName("bfmod.Taticas"), "SORTE", new Random());
      setStatic(Collections.class, "r", null);
      for (String s : new String[]{"gs", "gt", "gu", "gv", "gw"}) setStatic(I.class, s, null);
      for (int i = 0; i < MOD_RANDOMS.length; i++) {
         setStatic(Class.forName("bfmod." + MOD_RANDOMS[i]), "SORTE", new Random(900000L + i));
      }
   }

   // ---- the text both sides write ----

   final StringBuilder out = new StringBuilder();

   void line(String s) {
      out.append(s).append('\n');
   }

   static String n(Object o) {
      if (o == null) return "-";
      if (o instanceof F p) return p.getNome();
      if (o instanceof ah c) return c.getNome();
      if (o instanceof best.at c) return c.getNome();
      return o.toString();
   }

   static String ints(int[] a) {
      StringBuilder b = new StringBuilder();
      for (int i = 0; i < a.length; i++) b.append(i == 0 ? "" : ",").append(a[i]);
      return b.toString();
   }

   static String names(List<?> l) {
      StringBuilder b = new StringBuilder();
      for (Object o : l) b.append(" ").append(n(o)).append('#').append(((F) o).fT());
      return b.toString();
   }

   static String bits(double d) {
      return Long.toHexString(Double.doubleToRawLongBits(d));
   }

   void dumpMatch(int index, I m) throws Exception {
      line("M " + index + " " + n(m.hc()) + "|" + n(m.hd()) + " kind=" + m.hy().b() + "/" + m.hy().el());
      line("score " + m.hu() + " " + m.hw() + " added " + ints(m.ib()) + " winner " + n(m.hR()) + " shootout " + ints(m.hQ())
         + " flags " + get(m, "ge") + "," + get(m, "gf") + "," + get(m, "gj") + " gg " + m.hK() + " gh " + m.hL());
      line("stats possession=" + ints(m.hz()) + " shots=" + ints(m.hA()) + " on_target=" + ints(m.hZ()) + " wide=" + ints(m.ia())
         + " tackles=" + ints(m.hB()) + " pass_errors=" + ints(m.hC()) + " duels=" + ints(m.hY()) + " attendance=" + ints(m.hT())
         + " income=" + m.hN() + " subs_left=" + ints((int[]) get(m, "fR")) + " incidents=" + get(m, "fS") + "," + get(m, "fT") + "," + get(m, "fU"));
      line("tactics home=" + ints(m.hc().kj()) + " away=" + ints(m.hd().kj()));
      for (Object o : m.hE()) {
         best.A e = (best.A) o;
         line("E " + e.b() + " " + e.el() + " " + e.em() + " " + e.en() + " side=" + e.et() + " " + n(e.eo()) + " " + n(e.ep()) + " " + n(e.cu())
            + " flag=" + e.eu());
      }
      line("L home_starters" + names(m.hl()));
      line("L away_starters" + names(m.hm()));
      line("L home_bench" + names(m.hn()));
      line("L away_bench" + names(m.ho()));
      line("L home_played" + names(m.hp()));
      line("L away_played" + names(m.hq()));
      line("L home_came_on" + names(m.ie()));
      line("L away_came_on" + names((List<?>) get(m, "fM")));
   }

   /** The mod's tactics state of a match and its weights (bfmod.Taticas), for debugging. */
   void dumpTactics(I m) throws Exception {
      Class<?> taticas = Class.forName("bfmod.Taticas");
      Field partidas = taticas.getDeclaredField("partidas");
      partidas.setAccessible(true);
      Object p = ((Map<?, ?>) partidas.get(null)).get(m);
      if (p == null) {
         line("T none");
         return;
      }
      double[] meio = (double[]) get(p, "meio");
      double[] fm = (double[]) get(p, "fitMeio");
      double[] fv = (double[]) get(p, "fitVel");
      double[] fd = (double[]) get(p, "fitDes");
      line("T meio=" + bits(meio[0]) + "," + bits(meio[1]) + " fits=" + bits(fm[0]) + "," + bits(fm[1]) + "," + bits(fv[0]) + "," + bits(fv[1]) + ","
         + bits(fd[0]) + "," + bits(fd[1]));
      StringBuilder w = new StringBuilder("W");
      for (String fn : new String[]{"meio", "chutar", "bloquear", "goleiro"}) {
         Method mt = taticas.getMethod(fn, I.class, int.class);
         for (int side = 0; side < 2; side++) w.append(' ').append(fn).append(side).append('=').append(bits((double) mt.invoke(null, m, side)));
      }
      Method setores = taticas.getMethod("setores", I.class, int.class, double.class, double.class);
      for (int side = 0; side < 2; side++) {
         double[] r = (double[]) setores.invoke(null, m, side, 3.0, 2.5);
         w.append(" setores").append(side).append('=').append(bits(r[0])).append(',').append(bits(r[1]));
      }
      line(w.toString());
      Method valor = Class.forName("bfmod.Motor").getDeclaredMethod("valor", F.class, int.class);
      valor.setAccessible(true);
      StringBuilder v = new StringBuilder("V");
      for (Object o : m.hl()) {
         F f = (F) o;
         v.append(' ').append(f.fT()).append(':').append(valor.invoke(null, o, f.fT())).append('[').append(f.getPosicao()).append(' ')
            .append(f.gK()).append(',').append(f.gJ()).append(',').append(f.gL()).append(',').append(f.gM()).append(',').append(f.gN()).append(',')
            .append(f.gO()).append(',').append(f.gP()).append(']');
      }
      line(v.toString());
   }

   void dumpPlayers() throws Exception {
      for (Object co : c.a.SR.P()) {
         ah club = (ah) co;
         line("C " + n(club) + " captain=" + n(club.kd()) + " penalties=" + n(club.ke()) + " free_kicks=" + n(club.lq()) + " lineup_set="
            + club.kf() + " starters" + names(club.kY()) + " bench" + names(club.kZ()));
         for (Object po : club.kc()) {
            F p = (F) po;
            components.s s = p.gB();
            line("P " + n(p) + " slot=" + p.fT() + " fitness=" + p.fp() + " strength=" + p.fi() + " injured=" + p.fo() + " started=" + p.fs()
               + " cards=" + p.gi() + " match=" + s.tu() + "," + s.tK() + "," + s.tv() + "," + s.tw() + "," + s.y() + "," + s.tx() + ","
               + s.ty() + "," + s.tz() + "," + s.tA() + "," + s.cD() + "," + s.tI());
            for (Object ho : p.gH()) {
               best.H h = (best.H) ho;
               line("  H " + n(h.gS()) + " " + h.w() + " " + h.y() + " " + ints(h.gW()) + " " + h.cD() + " " + h.gZ() + " " + bits(h.F()));
            }
            for (Object ko : p.gr()) {
               best.k k = (best.k) ko;
               line("  K " + k.H() + " " + k.ct() + " " + k.w() + " " + k.y() + " " + k.cv() + " " + k.cw() + " " + k.cD() + " " + k.cx() + " "
                  + k.cF() + " " + bits(k.F()));
            }
         }
      }
   }

   public static void main(String[] args) throws Exception {
      Path outDir = Paths.get(args[0]);
      boolean tactics = args[1].equals("1");
      long firstSeed = Long.parseLong(args[2]);
      Files.write(Paths.get(System.getProperty("user.dir"), "bfmod.config"),
         ("TATICAS=" + (tactics ? 1 : 0) + "\nTATICA_AUTO=0\nSATELITES=0\nPATRIMONIO=0\n").getBytes(StandardCharsets.UTF_8));
      Throwable[] error = new Throwable[1];
      Thread t = new Thread(null, () -> {
         try {
            new best.M(true);
            best.n.cU();
            EngineHarness h = new EngineHarness();
            // the world itself is drawn from the counter too, so every run builds the same one
            for (String c : STATIC_RANDOMS) Class.forName(c, true, EngineHarness.class.getClassLoader());
            Random.class.getMethod("bfSetNext", long.class).invoke(null, 1L);
            best.aq.tI = new Random();
            h.build();
            String world = outDir.resolve("engine-world.s22").toString();
            write(world);
            load(world);
            resetRandomness(firstSeed);
            List<I> matches = new ArrayList<>();
            for (Object o : ((best.a) c.a.SR.R().get(0)).h()) matches.add((I) o);
            for (int i = 0; i < matches.size(); i++) {
               I m = matches.get(i);
               m.hi();
               ah.a(m.hc(), m, 1, -1, true);
               ah.a(m.hd(), m, 2, -1, true);
               m.V();
               h.dumpMatch(i, m);
               if (System.getenv("BF_TRACE_TACTICS") != null) h.dumpTactics(m);
            }
            h.dumpPlayers();
            Files.write(outDir.resolve("engine-" + (tactics ? 1 : 0) + ".txt"), h.out.toString().getBytes(StandardCharsets.UTF_8));
         } catch (Throwable e) {
            error[0] = e;
         }
      }, "engine-harness", 1L << 30);
      t.start();
      t.join();
      Random.class.getMethod("bfSetNext", long.class).invoke(null, Long.MIN_VALUE);
      if (error[0] != null) {
         error[0].printStackTrace();
         System.exit(1);
      }
   }
}
