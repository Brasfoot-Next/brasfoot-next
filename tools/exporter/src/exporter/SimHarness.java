package exporter;

import com.esotericsoftware.kryo.Kryo;
import com.esotericsoftware.kryo.io.Input;
import com.esotericsoftware.kryo.io.Output;
import java.awt.Color;
import java.io.*;
import java.nio.file.*;
import java.util.*;

/**
 * Differential reference for the Rust simulation (crates/simulation). Creates a fictional career
 * through the game's own new-game code (p.dd/p.dg with made-up team templates instead of the
 * teams/ folder), saves it like the game (sim-start.s22), loads it back like the game, then plays
 * days headless (c.a.SR.V(), the UI flag bs off) with every no-arg Random drawing from a counter,
 * and saves the result (sim-after-<days>.s22). The Rust test simulates the same days from the
 * first save and compares its world with the second.
 *
 * Run with user.dir = an empty folder (no league configuration files: every country uses the
 * game's defaults), the mod classes first on the classpath and the patched Random.
 * Usage: SimHarness <out-dir> <days> <first seed>, or SimHarness <out-dir> human <first seed> for the
 * same career with a made-up human coach hired at a club (human-start.s22), saved before any day.
 */
public class SimHarness {
   /** {country id, clubs}: France (P72) and Portugal (P154). */
   /** BF_WORLD: a career with the international club cups and every national team. */
   static final boolean WORLD = System.getenv("BF_WORLD") != null;
   /** {country, clubs}: France and Portugal; with BF_WORLD, ten countries of five confederations. */
   static final int[][] COUNTRIES = System.getenv("BF_COUNTRIES") != null ? countries(System.getenv("BF_COUNTRIES"))
      : WORLD
      ? new int[][]{{72, 40}, {154, 20}, {65, 20}, {29, 20}, {11, 16}, {131, 12}, {68, 12}, {107, 12}, {57, 12}, {129, 12}}
      : new int[][]{{72, 40}, {154, 20}};

   /** BF_COUNTRIES="3:36,97:92": the leagues' countries and clubs instead (the formats with playoffs). */
   static int[][] countries(String spec) {
      String[] parts = spec.split(",");
      int[][] out = new int[parts.length][];
      for (int i = 0; i < parts.length; i++) {
         String[] kv = parts[i].split(":");
         out[i] = new int[]{Integer.parseInt(kv[0]), Integer.parseInt(kv[1])};
      }
      return out;
   }
   /**
    * BF_WORLD: {country, clubs} of the countries without a league that the continental cups take
    * places from (f.r, f.t, f.u, f.v, f.w, f.x, f.y, f.H, f.J), with a club or two to spare.
    */
   static final int[][] OUTSIDE_LEAGUES = WORLD ? new int[][]{{1, 4}, {2, 6}, {3, 7}, {4, 5}, {5, 4}, {9, 5}, {10, 5}, {12, 6}, {14, 4}, {15, 7}, {16, 6}, {21, 7}, {22, 4}, {25, 6}, {26, 10}, {27, 6}, {31, 6}, {36, 4}, {38, 4}, {39, 5}, {40, 6}, {42, 10}, {43, 4}, {44, 7}, {46, 10}, {47, 4}, {48, 4}, {49, 5}, {50, 4}, {51, 4}, {52, 6}, {53, 4}, {54, 7}, {58, 4}, {59, 5}, {60, 10}, {62, 7}, {63, 6}, {64, 6}, {66, 5}, {67, 4}, {69, 4}, {70, 6}, {71, 4}, {75, 4}, {76, 6}, {78, 6}, {79, 4}, {81, 4}, {84, 4}, {85, 7}, {86, 4}, {87, 4}, {88, 6}, {92, 6}, {93, 4}, {95, 4}, {96, 4}, {97, 9}, {98, 5}, {99, 4}, {100, 6}, {101, 6}, {102, 5}, {103, 6}, {104, 9}, {105, 6}, {106, 4}, {108, 4}, {110, 6}, {114, 6}, {115, 4}, {116, 4}, {118, 4}, {119, 6}, {120, 6}, {122, 6}, {124, 4}, {127, 4}, {128, 6}, {132, 4}, {133, 4}, {134, 6}, {139, 4}, {141, 4}, {142, 6}, {143, 4}, {145, 5}, {147, 4}, {148, 4}, {150, 10}, {151, 10}, {152, 6}, {157, 4}, {158, 4}, {159, 6}, {160, 6}, {164, 5}, {169, 4}, {171, 6}, {173, 4}, {174, 4}, {178, 4}, {179, 6}, {180, 6}, {182, 4}, {183, 4}, {184, 4}, {186, 4}, {189, 4}, {190, 4}, {192, 7}, {193, 7}, {194, 4}, {195, 10}, {196, 4}, {197, 4}, {198, 10}, {199, 4}, {200, 4}, {201, 4}, {215, 4}, {216, 6}} : new int[0][];
   static final String[] CLUB_WORDS = {"Atlético", "União", "Esporte Clube", "Sporting", "Racing", "Olympique", "Estrela", "Real"};
   static final String[] TOWNS = {"Fictícia", "Imaginária", "dos Testes", "Inventada", "Hipotética", "de Faz de Conta", "Ilusória", "do Exemplo"};

   final Random setup = new Random(20261021L);

   /** The game alone: every mod system off but the tactics (each system gets its own test). */
   static final String SYSTEMS_OFF = "TATICAS=1\nTATICA_AUTO=0\nJORNAL=0\nEFEITOS_PREMIOS=0\nPROPOSTAS=0\nJANELA_USUARIO=0\n"
      + "EMPRESTIMOS_IA=0\nPEDIDOS_EMPRESTIMO=0\nBASE=0\nPATROCINIO=0\nINVESTIDOR=0\nTORCIDA=0\nCOMISSAO=0\nCAMPEONATO_BASE=0\n"
      + "PATRIMONIO=0\nESTADIOS=0\nCREDITO=0\nSATELITES=0\nMERCADO=DESLIGADO\n";

   /** BF_SYSTEMS: a comma-separated list of the mod's switches to turn on (BASE, COMISSAO, ...). */
   static final String[] SYSTEMS_ON = System.getenv("BF_SYSTEMS") == null ? new String[0] : System.getenv("BF_SYSTEMS").split(",");

   /** A config with the BF_SYSTEMS switches on. */
   static String withSystems(String config) {
      for (String k : SYSTEMS_ON) config = config.replace("\n" + k + "=0", "\n" + k + "=1");
      return config;
   }

   /** The fixtures' prefix part for BF_SYSTEMS: "base-comissao-" for BASE,COMISSAO. */
   static String systemsPrefix() {
      StringBuilder b = new StringBuilder();
      for (String k : SYSTEMS_ON) b.append(k.toLowerCase()).append('-');
      return b.toString();
   }

   /** SYSTEMS_OFF with the awards' effects on (players become stars, the coach of the year gains reputation). */
   static final String AWARDS_ON = SYSTEMS_OFF.replace("EFEITOS_PREMIOS=0", "EFEITOS_PREMIOS=1");

   /** The side files the awards keep beside the save (Premios, HallDaFama, Clubes). */
   static final String[] AWARD_FILES = {".premios", ".carreiras", ".recordes", ".clubes", ".idolos"};

   /** The side files of the mod's clock (Economia, Janelas). */
   static final String[] CLOCK_FILES = {".economia", ".janelas", ".comissao", ".base", ".joias", ".torcida", ".patrocinio", ".cotas", ".investidor", ".patrimonio", ".estadios", ".satelites", ".credito", ".emprestimos", ".supermundial", ".base_camp", ".base_hist"};

   /**
    * BolaDeOuro.tick without its window parts (Diagnostico, Taticas.instalar, Menu.instalar): the
    * systems' ticks in its order, each exception printed and swallowed as there.
    */
   static void modTick() throws Exception {
      String[] ticks = {"Estadios", "Base", "Jornal", "Economia", "Olheiros", "Torcida", "Mensal", "Patrocinio", "Investidor", "Cotas", "Caixa",
         "Emprestimos", "SuperMundial"};
      call("Taticas", "prepararProximo");
      for (String c : ticks) {
         HumanHarness.checkpoint("mod " + c);
         call(c, "tick");
      }
      HumanHarness.checkpoint("mod end");
      // what the ticks left for the event queue (the mod's dialogs, answered by HumanPolicy)
      HumanPolicy.runLater();
   }

   static void call(String cls, String method) throws Exception {
      java.lang.reflect.Method m = Class.forName("bfmod." + cls).getDeclaredMethod(method);
      m.setAccessible(true);
      try {
         m.invoke(null);
      } catch (java.lang.reflect.InvocationTargetException e) {
         e.getCause().printStackTrace();
      }
   }

   e.g player(int country, int position, boolean youth) {
      e.g p = new e.g();
      p.setNome("TESTE"); // F.setNome draws a name from the game's list for its country
      p.setPais(country);
      p.setPosicao(position);
      p.setLado(setup.nextInt(2));
      int[] traits = best.F.at(position);
      p.setCr1(traits[0]);
      p.setCr2(traits[1]);
      p.setIdade(youth ? 16 + setup.nextInt(4) : 18 + setup.nextInt(17));
      p.setHash(1 + setup.nextInt(10));
      p.setStatus(setup.nextInt(10) == 0 ? 1 : 0);
      p.setEstrela(setup.nextInt(30) == 0);
      p.setTopMundial(false);
      return p;
   }

   e.t club(int country, int number, int id) {
      e.t t = new e.t();
      t.setValid(true);
      t.setId(id);
      t.setPais(country);
      t.setEstado(-1);
      t.setNivel(5 + setup.nextInt(18));
      t.setReputacao(number < 4 ? 4 : number < 12 ? 3 : number < 24 ? 2 : 1);
      String name = CLUB_WORDS[number % CLUB_WORDS.length] + " " + TOWNS[(number / CLUB_WORDS.length) % TOWNS.length] + " " + (number + 1);
      t.setNome(name);
      t.setFileRef("ficticio_" + id);
      t.setEstadio("Estádio " + TOWNS[number % TOWNS.length] + " " + (number + 1));
      t.setCapacidade(5000 + 1000 * setup.nextInt(60));
      t.setTecnico("Técnico Inventado " + id);
      t.setTecNac(country);
      t.setCorBase(setup.nextInt(10));
      t.setCor1(String.format("#%06x", setup.nextInt(0xffffff)));
      t.setCor2(String.format("#%06x", setup.nextInt(0xffffff)));
      t.setCorF(Color.decode(t.getCor1()));
      t.setCorT(Color.decode(t.getCor2()));
      ArrayList<e.g> squad = new ArrayList<>();
      int[] positions = {0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4};
      for (int position : positions) squad.add(player(country, position, false));
      t.setJogadores(squad);
      ArrayList<e.g> youth = new ArrayList<>();
      for (int i = 0; i < 5; i++) youth.add(player(country, 1 + setup.nextInt(4), true));
      t.setJuniores(youth);
      return t;
   }

   /**
    * With BF_TEAMS_DIR, each made-up team is also written as a team file ("0001_ficticio_1.ban",
    * in creation order), for the Rust new game to start from the same teams.
    */
   static void writeTeam(e.t team, int number) throws Exception {
      String dir = System.getenv("BF_TEAMS_DIR");
      if (dir == null) return;
      Path file = Paths.get(dir, String.format("%04d_%s.ban", number, team.getFileRef()));
      try (ObjectOutputStream out = new ObjectOutputStream(new FileOutputStream(file.toFile()))) {
         out.writeObject(team);
      }
   }

   /** p.dd() and the league choice of the new-game screens, with made-up templates. */
   void newGame() throws Exception {
      c.a.SR = new best.f();
      c.a.afQ = new best.ay();
      est.Options options = c.a.vM();
      options.setJogaEstadual(false);
      options.setJogaIntClubes(WORLD);
      options.setJogaSelecoesAll(WORLD);
      options.setJogaRegionais(false);
      options.setHabilidadeIndividual(true);
      options.setSalarioMensal(true);
      options.setAutoSalvar(0);
      new best.M(false);
      c.a.SR.I(2022);
      c.a.SR.setJogaEstadual(false);
      c.a.SR.setJogaIntClubes(WORLD);
      c.a.SR.setJogaSelecoesAll(WORLD);
      c.a.SR.setJogaRegionais(false);
      best.p.de();
      c.a.SR.bC = new ArrayList();
      c.a.SR.bD = new int[best.ac.jz()];
      c.a.SR.bE = new int[best.aq.rX.length];
      int id = 1;
      for (int[] country : COUNTRIES) {
         for (int i = 0; i < country[1]; i++) {
            e.t team = club(country[0], i, id++);
            c.a.SR.bC.add(team);
            c.a.SR.bD[country[0]]++;
            writeTeam(team, c.a.SR.bC.size());
         }
      }
      // the clubs outside the leagues: team files only, no league (bF)
      for (int[] country : OUTSIDE_LEAGUES) {
         for (int i = 0; i < country[1]; i++) {
            e.t team = club(country[0], i, id++);
            c.a.SR.bC.add(team);
            c.a.SR.bD[country[0]]++;
            writeTeam(team, c.a.SR.bC.size());
         }
      }
      Collections.sort(c.a.SR.bC, best.p.cN);
      c.a.SR.bF = new ArrayList();
      for (int[] country : COUNTRIES) {
         best.O o = new best.O();
         o.setPais(country[0]);
         o.aX(country[1]);
         o.v(true);
         c.a.SR.bF.add(o);
      }
      c.a.SR.bG = new ArrayList();
      best.p.ds();
      best.p.dg();
      best.p.dh();
      c.a.SR.az();
   }

   /** best.al.c(String, int) creates a human coach; the game's own hiring (f.a(ah, al, al)) gives him club 1. */
   static void hireHuman() {
      best.al.c("Técnico Fictício", 72);
      best.al coach = (best.al) c.a.SR.M().get(c.a.SR.M().size() - 1);
      best.ah club = c.a.SR.x(1);
      c.a.SR.a(club, club.ka(), coach);
   }

   public static void main(String[] args) throws Exception {
      Path outDir = Paths.get(args[0]);
      // "awards:<days>": the same days with the mod's season-end awards (BolaDeOuro.fimDeTemporada)
      // "clock:<days>": the awards plus the mod's clock, one tick when the main screen comes back
      boolean clock = args[1].startsWith("clock:");
      boolean awards = clock || args[1].startsWith("awards:");
      int days = args[1].equals("human") || args[1].equals("coach-screen") ? 0 : Integer.parseInt(awards ? args[1].substring(args[1].indexOf(':') + 1) : args[1]);
      long firstSeed = Long.parseLong(args[2]);
      // BF_MARKET: the mod's market intensity (CONTIDO, REALISTA, AGITADO) instead of off
      String market = System.getenv("BF_MARKET");
      String config = withSystems(awards ? AWARDS_ON : SYSTEMS_OFF);
      if (market != null) config = config.replace("MERCADO=DESLIGADO", "MERCADO=" + market);
      Files.write(Paths.get(System.getProperty("user.dir"), "bfmod.config"), config.getBytes());
      Throwable[] error = new Throwable[1];
      Thread t = new Thread(null, () -> {
         try {
            new best.M(true);
            best.n.cU();
            for (String c : EngineHarness.STATIC_RANDOMS) Class.forName(c, true, SimHarness.class.getClassLoader());
            Random.class.getMethod("bfSetNext", long.class).invoke(null, 1L);
            // the calendar days take the time of day of "now": a fixed one, 2026-10-20 14:30:15.250 UTC
            GregorianCalendar.class.getMethod("bfSetNow", long.class).invoke(null, 1792506615250L);
            best.aq.tI = new Random();
            new SimHarness().newGame();
            if (args[1].equals("coach-screen")) {
               // the new game's coach screen (a.bV.or()): the made-up human coach takes club 1
               c.a.SR.x(1).b("Técnico Fictício", 72);
               EngineHarness.write(outDir.resolve("coach-screen-start.s22").toString());
               return;
            }
            if (args[1].equals("human")) {
               hireHuman();
               EngineHarness.write(outDir.resolve("human-start.s22").toString());
               System.out.println("human coach at " + c.a.SR.x(1).getNome());
               return;
            }
            String start = outDir.resolve((System.getenv("BF_FIXTURE_PREFIX") != null ? System.getenv("BF_FIXTURE_PREFIX") : WORLD ? "world-" : "") + "sim-start.s22").toString();
            EngineHarness.write(start);
            EngineHarness.load(start);
            EngineHarness.resetRandomness(firstSeed);
            HumanHarness.checkpoint("start");
            c.a.SR.bs = false;
            // the loaded career opens on the main screen
            if (clock) modTick();
            for (int day = 0; day < days; day++) {
               HumanHarness.checkpoint("day " + day);
               boolean over = c.a.SR.bk;
               c.a.SR.V();
               // with the screens on, f.ax() opens the season-end screen a.cR (S.aY(6)) right after
               // f.al(), and its constructor hands the season to the mod; any other day ends on the
               // main screen
               if (awards && !over && c.a.SR.bk) bfmod.BolaDeOuro.fimDeTemporada(null, false);
               else if (clock) modTick();
            }
            HumanHarness.checkpoint("end");
            String prefix = (System.getenv("BF_FIXTURE_PREFIX") != null ? System.getenv("BF_FIXTURE_PREFIX") : WORLD ? "world-" : "") + (clock ? "clock-" : awards ? "awards-" : "sim-") + systemsPrefix() + (market != null ? "market-" : "") + "after-";
            EngineHarness.write(outDir.resolve(prefix + days + ".s22").toString());
            if (awards) {
               String save = c.a.SR.br() == null || c.a.SR.br().isEmpty() ? "sem_nome" : c.a.SR.br();
               List<String> files = new ArrayList<>(Arrays.asList(AWARD_FILES));
               if (clock) files.addAll(Arrays.asList(CLOCK_FILES));
               for (String ext : files) {
                  Path file = Paths.get(System.getProperty("user.dir"), "sav", save + ext);
                  Path copy = outDir.resolve(prefix + days + ext);
                  if (Files.exists(file)) Files.copy(file, copy, StandardCopyOption.REPLACE_EXISTING);
                  else Files.deleteIfExists(copy);
               }
            }
            System.out.println("day " + c.a.SR.J() + " season " + c.a.SR.H() + " clubs " + c.a.SR.P().size() + " players " + c.a.SR.O().size());
         } catch (Throwable e) {
            error[0] = e;
         }
      }, "sim-harness", 1L << 30);
      t.start();
      t.join();
      if (error[0] != null) {
         error[0].printStackTrace();
         System.exit(1);
      }
   }
}
