package exporter;

import java.lang.reflect.Field;
import java.nio.file.*;
import java.util.*;

/**
 * Differential reference for a career with a human club. The fictional career of SimHarness with
 * its made-up human coach at club 1 (human-start.s22), saved and loaded like the game, then days
 * played the way the game plays them with its screens: on a day the human club plays, the human
 * confirms a line-up in a.bz, the AI picks its line-ups (ah.kh()), the AI-only matches are played
 * (I.he()) and the round screen a.dV plays the human's match minute by minute and shows the others
 * (best.s.dx()), then f.at(). The round screen is a copy of a.dV without its timer
 * (tools/exporter/human-patch) and the dialogs follow HumanPolicy. Every no-arg Random draws from a
 * counter. Writes human-after-<days>.s22.
 *
 * Usage: HumanHarness <out-dir> <days> <first seed>, with user.dir an empty folder, the patched
 * a.dV, the mod classes and the patched Random on the classpath (export.sh human-days).
 */
public class HumanHarness {
   static Object getStatic(Class<?> c, String name) throws Exception {
      Field f = c.getDeclaredField(name);
      f.setAccessible(true);
      return f.get(null);
   }

   static boolean humanPlays(best.I m) {
      return m.hc().jZ() || m.hd().jZ();
   }

   static final boolean TRACE = System.getenv("BF_TRACE_RNG") != null;

   /** With BF_TRACE_RNG, prints a label and the next seed of the counter (Runtime::checkpoint on the Rust side). */
   public static void checkpoint(String label) throws Exception {
      if (!TRACE) return;
      Field f = Random.class.getDeclaredField("bfNext");
      f.setAccessible(true);
      System.out.println("rng " + label + " " + f.getLong(null) + " draws " + Random.class.getField("bfDraws").getLong(null));
   }

   /** One c.a.SR.V() as the game runs it with the screens, the human confirming his line-ups. */
   static void playDay() throws Exception {
      best.f game = c.a.SR;
      if (game.bk) {
         game.V();
         return;
      }
      ArrayList<?> today = game.S();
      List<best.I> human = new ArrayList<>();
      for (Object o : today) {
         if (humanPlays((best.I) o)) human.add((best.I) o);
      }
      if (human.isEmpty()) {
         game.V();
         return;
      }
      if (HumanPolicy.STAFF) HumanPolicy.hireStaff();
      if (HumanPolicy.SPONSORS) HumanPolicy.sponsorSlots();
      if (HumanPolicy.ASSETS) for (best.I m : human) for (best.ah club : new best.ah[]{m.hc(), m.hd()}) if (club.jZ()) HumanPolicy.investInAssets(club);
      if (HumanPolicy.BANK) for (best.I m : human) for (best.ah club : new best.ah[]{m.hc(), m.hd()}) if (club.jZ()) HumanPolicy.bank(club);
      if (HumanPolicy.SATELLITES) for (best.I m : human) for (best.ah club : new best.ah[]{m.hc(), m.hd()}) if (club.jZ()) HumanPolicy.satellites(club);
      if (HumanPolicy.JUNIORS) for (best.I m : human) for (best.ah club : new best.ah[]{m.hc(), m.hd()}) if (club.jZ()) HumanPolicy.promoteJuniors(club);
      for (best.I m : human) {
         for (best.ah club : new best.ah[]{m.hc(), m.hd()}) {
            if (club.jZ() && !club.kf()) {
               checkpoint("contracts");
               HumanPolicy.renewContracts(club);
               checkpoint("line-up");
               if (System.getenv("BF_TRACE_LINEUP") != null) Random.class.getField("bfTrace").setBoolean(null, true);
               HumanPolicy.lineUp(club, m);
               Random.class.getField("bfTrace").setBoolean(null, false);
            }
         }
      }
      // f.ap() once f.ay() finds every human line-up confirmed (bH: no competition is skipped here)
      game.bH = false;
      checkpoint("day");
      best.ah.kh();
      checkpoint("ai line-ups");
      best.I.he();
      checkpoint("ai matches");
      if (best.s.dw()) {
         a.dV round = new a.dV(null, 700);
         EngineHarness.setStatic(best.s.class, "cR", round);
         best.s.dx();
         while (getStatic(best.s.class, "cR") != null) {
            checkpoint("tick " + getStatic(a.dV.class, "dB") + " " + getStatic(a.dV.class, "GT"));
            round.qa();
         }
      } else {
         game.at();
      }
      if (game.bL) game.bL = false;
   }

   public static void main(String[] args) throws Exception {
      Path outDir = Paths.get(args[0]);
      int days = Integer.parseInt(args[1]);
      long firstSeed = Long.parseLong(args[2]);
      // BF_CLOCK: the awards at each season end and the mod's clock after every other day, with the
      // market of BF_MARKET (off by default)
      boolean clock = System.getenv("BF_CLOCK") != null;
      String market = System.getenv("BF_MARKET");
      String config = SimHarness.withSystems(clock ? SimHarness.AWARDS_ON : SimHarness.SYSTEMS_OFF);
      if (market != null) config = config.replace("MERCADO=DESLIGADO", "MERCADO=" + market);
      // BF_STAFF: the coaching staff and the automatic tactics on, the human hiring (HumanPolicy.hireStaff)
      if (HumanPolicy.STAFF) config = config.replace("COMISSAO=0", "COMISSAO=1").replace("TATICA_AUTO=0", "TATICA_AUTO=1");
      Files.write(Paths.get(System.getProperty("user.dir"), "bfmod.config"), config.getBytes());
      Throwable[] error = new Throwable[1];
      Thread t = new Thread(null, () -> {
         try {
            new best.M(true);
            best.n.cU();
            for (String c : EngineHarness.STATIC_RANDOMS) Class.forName(c, true, HumanHarness.class.getClassLoader());
            Random.class.getMethod("bfSetNext", long.class).invoke(null, 1L);
            GregorianCalendar.class.getMethod("bfSetNow", long.class).invoke(null, 1792506615250L);
            best.aq.tI = new Random();
            new SimHarness().newGame();
            SimHarness.hireHuman();
            String start = outDir.resolve("human-start.s22").toString();
            EngineHarness.write(start);
            EngineHarness.load(start);
            // the screens' first fonts load ThreadLocalRandom, whose class creates a no-arg
            // Random: do it before the counter starts, as the game does long before a match
            java.util.concurrent.ThreadLocalRandom.current();
            new javax.swing.JLabel("x").getFontMetrics(new java.awt.Font("Dialog", java.awt.Font.PLAIN, 12));
            EngineHarness.resetRandomness(firstSeed);
            c.a.SR.bs = false;
            if (clock) SimHarness.modTick();
            for (int day = 0; day < days; day++) {
               boolean over = c.a.SR.bk;
               playDay();
               // BF_INVESTOR_DAY: the mod's test command "investidor" before that day's tick
               if (System.getenv("BF_INVESTOR_DAY") != null && day + 1 == Integer.parseInt(System.getenv("BF_INVESTOR_DAY"))) {
                  java.lang.reflect.Field f = Class.forName("bfmod.Investidor").getDeclaredField("pedidoDeTeste");
                  f.setAccessible(true);
                  f.setBoolean(null, true);
               }
               if (clock) {
                  if (!over && c.a.SR.bk) bfmod.BolaDeOuro.fimDeTemporada(null, false);
                  else SimHarness.modTick();
               }
               if (System.getenv("BF_TRACE_DAYS") != null) {
                  best.al human = null;
                  for (Object o : c.a.SR.M()) if (((best.al) o).jZ()) human = (best.al) o;
                  // BF_TRACE_COACH: that coach instead of the human
                  if (System.getenv("BF_TRACE_COACH") != null) for (Object o : c.a.SR.L()) if (((best.al) o).dS().endsWith(System.getenv("BF_TRACE_COACH"))) human = (best.al) o;
                  System.out.println("after " + (day + 1) + " days: season " + c.a.SR.H() + ", calendar day " + c.a.SR.J()
                     + ", human coach at " + (human == null || human.fg() == null ? "-" : human.fg().getNome())
                     + (human == null ? "" : ", board " + human.lL() + ", fans " + human.lM() + (human.fg() == null ? "" : ", cash " + human.fg().kb())));
               }
            }
            String prefix = "human-" + (clock ? "clock-" : "") + SimHarness.systemsPrefix() + (HumanPolicy.STAFF ? "staff-" : "") + (HumanPolicy.JUNIORS ? "juniors-" : "") + (HumanPolicy.SPONSORS ? "sponsors-" : "") + (HumanPolicy.ASSETS ? "assets-" : "") + (HumanPolicy.BANK ? "bank-" : "") + (HumanPolicy.SATELLITES ? "satellites-" : "") + (System.getenv("BF_INVESTOR_DAY") != null ? "investor" + System.getenv("BF_INVESTOR_DAY") + "-" : "") + (market != null ? "market-" : "") + "after-";
            EngineHarness.write(outDir.resolve(prefix + days + ".s22").toString());
            if (clock) {
               String save = c.a.SR.br() == null || c.a.SR.br().isEmpty() ? "sem_nome" : c.a.SR.br();
               List<String> files = new ArrayList<>(Arrays.asList(SimHarness.AWARD_FILES));
               files.addAll(Arrays.asList(SimHarness.CLOCK_FILES));
               for (String ext : files) {
                  Path file = Paths.get(System.getProperty("user.dir"), "sav", save + ext);
                  Path copy = outDir.resolve(prefix + days + ext);
                  if (Files.exists(file)) Files.copy(file, copy, StandardCopyOption.REPLACE_EXISTING);
               }
            }
            System.out.println("day " + c.a.SR.J() + " season " + c.a.SR.H() + " dialogs " + HumanPolicy.DIALOGS + " renewals " + HumanPolicy.RENEWALS + " jobs taken " + HumanPolicy.JOBS_TAKEN + " staff hired " + HumanPolicy.HIRES + " juniors promoted " + HumanPolicy.PROMOTED + " masters " + HumanPolicy.MASTERS + " investors " + HumanPolicy.INVESTORS + " slots signed " + HumanPolicy.SLOTS_SIGNED + " asset actions " + HumanPolicy.ASSET_ACTIONS + " squad offers " + HumanPolicy.SQUAD_OFFERS);
         } catch (Throwable e) {
            error[0] = e;
         }
      }, "human-harness", 1L << 30);
      t.start();
      t.join();
      if (error[0] != null) {
         error[0].printStackTrace();
         System.exit(1);
      }
   }
}
