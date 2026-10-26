package exporter;

import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Method;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.TreeMap;

/**
 * What the human of exporter.HumanHarness does at each decision the game asks with a dialog: he
 * never changes anything and confirms. Each dialog's own class is built without a window and its
 * steps run in the order its timers or buttons would run them, so the game logic inside the
 * dialogs (the penalty in a.gb, the shoot-out in a.bg) is the game's.
 */
public class HumanPolicy {
   /** Dialogs the harness's best.S was asked to open, with how many times. */
   static final Map<String, Integer> DIALOGS = new TreeMap<>();

   /** A dialog of best.S: the human closes it without accepting anything. */
   public static void dialog(String name) {
      DIALOGS.merge(name, 1, Integer::sum);
      System.out.println("dialog " + name + " on day " + c.a.SR.J() + " season " + c.a.SR.H());
   }

   /** With BF_STAFF, the human hires a coaching staff (bfmod.Comissao). */
   static final boolean STAFF = System.getenv("BF_STAFF") != null;
   /** STAFF_LEVEL: the lowest level the human hires. */
   static final int STAFF_LEVEL = 3;
   static int HIRES = 0;

   /**
    * bfmod.Comissao's screen: for each role (in Cargo order) without someone of STAFF_LEVEL or more,
    * the first candidate of the month at that level who accepts the club.
    */
   static void hireStaff() throws Exception {
      Class<?> staff = Class.forName("bfmod.Comissao");
      Class<?> role = Class.forName("bfmod.Comissao$Cargo");
      Method hired = staff.getDeclaredMethod("contratado", role);
      Method candidates = staff.getDeclaredMethod("candidatos");
      Method hire = staff.getDeclaredMethod("contratar", Class.forName("bfmod.Comissao$Profissional"));
      for (Method m : new Method[]{hired, candidates, hire}) m.setAccessible(true);
      for (Object r : role.getEnumConstants()) {
         Object current = hired.invoke(null, r);
         if (current != null && (int) EngineHarness.get(current, "nivel") >= STAFF_LEVEL) continue;
         for (Object p : (List<?>) candidates.invoke(null)) {
            if (EngineHarness.get(p, "cargo") != r || (int) EngineHarness.get(p, "nivel") < STAFF_LEVEL) continue;
            if (hire.invoke(null, p) == null) {
               HIRES++;
               break;
            }
         }
      }
   }

   /** With BF_JUNIORS, the human promotes his juniors of 18 or more (bfmod.BaseDialog). */
   static final boolean JUNIORS = System.getenv("BF_JUNIORS") != null;
   static int PROMOTED = 0;

   /** BaseDialog's "Promover ao Profissional" for every junior of 18 or more, in the list's order, while the squad has room. */
   static void promoteJuniors(best.ah club) throws Exception {
      Method promoted = Class.forName("bfmod.Base").getDeclaredMethod("aoPromover", best.F.class);
      promoted.setAccessible(true);
      for (Object o : new ArrayList<Object>(c.a.SR.Q())) {
         best.F p = (best.F) o;
         if (p.fg() != club || p.getIdade() < 18) continue;
         if (club.kc() != null && club.kc().size() >= 35) return;
         best.K.a(false, p, club);
         p.aB(300);
         promoted.invoke(null, p);
         PROMOTED++;
      }
   }

   /** The mod's dialogs opened with invokeLater (tools/exporter/mod-patch), run after the tick. */
   static final List<Runnable> LATER = new ArrayList<>();

   public static void later(Runnable r) {
      LATER.add(r);
   }

   static void runLater() {
      while (!LATER.isEmpty()) LATER.remove(0).run();
   }

   /** Patrocinio's three master offers (fixed, mixed, risky): the human signs the mixed one. */
   public static int masterSponsor(int offers) {
      MASTERS++;
      return Math.min(1, offers - 1);
   }

   /** Investidor's offer to buy the club: the human accepts. */
   public static boolean acceptInvestor() {
      INVESTORS++;
      return true;
   }

   /** Mercado.mostrar: the human accepts every offer and loan request for his squad. */
   public static boolean acceptSquadOffer(boolean loan) {
      SQUAD_OFFERS++;
      return true;
   }

   static int SQUAD_OFFERS = 0;

   /** best.S.aY(2) after an accepted offer: the main screen again, with the mod's clock. */
   public static void mainScreenAgain() {
      try {
         SimHarness.modTick();
      } catch (Exception e) {
         throw new RuntimeException(e);
      }
   }

   static int MASTERS = 0;
   static int INVESTORS = 0;

   /** With BF_SPONSORS, the human works the sponsorship slots (bfmod.Cotas). */
   static final boolean SPONSORS = System.getenv("BF_SPONSORS") != null;
   static int SLOTS_SIGNED = 0;

   /**
    * The finances panel's slots, in Cota order: the first offer of a slot that has offers is
    * signed; an empty slot nobody is looking for gets the commercial department.
    */
   static void sponsorSlots() throws Exception {
      Class<?> slots = Class.forName("bfmod.Cotas");
      Class<?> slot = Class.forName("bfmod.Cotas$Cota");
      Class<?> deal = Class.forName("bfmod.Cotas$Contrato");
      Method offers = slots.getDeclaredMethod("ofertas", slot);
      Method contract = slots.getDeclaredMethod("contrato", slot);
      Method searching = slots.getDeclaredMethod("buscando", slot);
      Method search = slots.getDeclaredMethod("buscar", slot);
      Method sign = slots.getDeclaredMethod("assinar", deal);
      for (Method m : new Method[]{offers, contract, searching, search, sign}) m.setAccessible(true);
      for (Object c : slot.getEnumConstants()) {
         List<?> o = (List<?>) offers.invoke(null, c);
         if (!o.isEmpty()) {
            sign.invoke(null, o.get(0));
            SLOTS_SIGNED++;
         } else if (contract.invoke(null, c) == null && (int) searching.invoke(null, c) == 0) {
            search.invoke(null, c);
         }
      }
   }

   /** With BF_ASSETS, the human invests in the club's infrastructure (bfmod.Patrimonio). */
   static final boolean ASSETS = System.getenv("BF_ASSETS") != null;
   static int ASSET_ACTIONS = 0;

   /**
    * The stadium/assets panel: the first pillar (in Pilar order) whose next level costs at most
    * half the cash is built; with no work going on and over 60 mi in cash, a stadium expansion;
    * the first naming-rights offer when there is none; the national campaign when it is allowed
    * and costs at most a third of the cash.
    */
   static void investInAssets(best.ah club) throws Exception {
      Class<?> assets = Class.forName("bfmod.Patrimonio");
      Class<?> pillar = Class.forName("bfmod.Patrimonio$Pilar");
      Class<?> offer = Class.forName("bfmod.Patrimonio$PropostaNaming");
      Method level = assets.getDeclaredMethod("nivel", best.ah.class, pillar);
      Method upgrade = assets.getDeclaredMethod("evoluir", best.ah.class, pillar);
      Method expand = assets.getDeclaredMethod("iniciarExpansao", best.ah.class, int[].class);
      Method offers = assets.getDeclaredMethod("propostasNaming", best.ah.class);
      Method sign = assets.getDeclaredMethod("assinarNaming", best.ah.class, offer);
      Method data = assets.getDeclaredMethod("dados", best.ah.class);
      Method campaign = assets.getDeclaredMethod("realizarCampanha", best.ah.class, int.class);
      Method works = assets.getDeclaredMethod("obraEmAndamento", best.v.class);
      for (Method m : new Method[]{level, upgrade, expand, offers, sign, data, campaign, works}) m.setAccessible(true);
      long[] cost = (long[]) HumanHarness.getStatic(assets, "CUSTO_NIVEL");
      for (Object p : pillar.getEnumConstants()) {
         int n = (int) level.invoke(null, club, p);
         if (n < 5 && cost[n + 1] * 2 <= club.kb()) {
            if (upgrade.invoke(null, club, p) == null) ASSET_ACTIONS++;
            break;
         }
      }
      if (club.ev() != null && works.invoke(null, club.ev()) == null && club.kb() > 60_000_000L) {
         if (expand.invoke(null, club, new int[]{500, 2000, 300, 20}) == null) ASSET_ACTIONS++;
      }
      Object e = data.invoke(null, club);
      if (((String) EngineHarness.get(e, "marcaNaming")).isEmpty()) {
         List<?> o = (List<?>) offers.invoke(null, club);
         if (!o.isEmpty() && sign.invoke(null, club, o.get(0)) == null) ASSET_ACTIONS++;
      }
      if ((int) EngineHarness.get(e, "marketing") >= 1 && 20_000_000L * 3 <= club.kb()) {
         if (campaign.invoke(null, club, 1) == null) ASSET_ACTIONS++;
      }
   }

   /** With BF_BANK, the human borrows from the bank (bfmod.Credito): with no loan, 80% of the available limit over 12 months. */
   static final boolean BANK = System.getenv("BF_BANK") != null;
   static int BANK_LOANS = 0;

   static void bank(best.ah club) throws Exception {
      Class<?> credit = Class.forName("bfmod.Credito");
      Method loans = credit.getDeclaredMethod("emprestimosDe", best.ah.class);
      Method available = credit.getDeclaredMethod("limiteDisponivel", best.ah.class);
      Method borrow = credit.getDeclaredMethod("contratar", best.ah.class, long.class, int.class);
      for (Method m : new Method[]{loans, available, borrow}) m.setAccessible(true);
      if (((List<?>) loans.invoke(null, club)).isEmpty()) {
         long amount = (long) available.invoke(null, club) * 8 / 10;
         if (amount > 0 && borrow.invoke(null, club, amount, 12) != null) BANK_LOANS++;
      }
   }

   /**
    * With BF_SATELLITES, the human builds a network (bfmod.Satelites): with no satellite, a
    * partnership with the first AI club that may be one, outside his division; with a satellite and no loanee, his
    * youngest player not on loan goes there as a regular starter.
    */
   static final boolean SATELLITES = System.getenv("BF_SATELLITES") != null;
   static int SATELLITE_ACTIONS = 0;

   static void satellites(best.ah club) throws Exception {
      Class<?> sat = Class.forName("bfmod.Satelites");
      Class<?> role = Class.forName("bfmod.Satelites$PapelEmprestimo");
      Method of = sat.getDeclaredMethod("satelitesDe", best.ah.class);
      Method block = sat.getDeclaredMethod("motivoBloqueio", best.ah.class, best.ah.class, boolean.class);
      Method sign = sat.getDeclaredMethod("firmarParceria", best.ah.class, best.ah.class);
      Method lend = sat.getDeclaredMethod("emprestarParaSatelite", best.F.class, best.ah.class, role);
      Method same = sat.getDeclaredMethod("mesmaDivisao", best.ah.class, best.ah.class);
      Method ai = Class.forName("bfmod.Economia").getDeclaredMethod("clubesIA");
      Method loanOf = Class.forName("bfmod.Emprestimos").getDeclaredMethod("emprestimoDe", best.F.class);
      for (Method m : new Method[]{of, block, same, sign, lend, ai, loanOf}) m.setAccessible(true);
      List<?> mine = (List<?>) of.invoke(null, club);
      if (mine.isEmpty()) {
         for (Object t : (List<?>) ai.invoke(null)) {
            if (block.invoke(null, club, t, false) == null && !(boolean) same.invoke(null, club, t)) {
               if (sign.invoke(null, club, t) == null) SATELLITE_ACTIONS++;
               return;
            }
         }
         return;
      }
      best.ah satellite = (best.ah) mine.get(0);
      for (Object o : satellite.kc()) {
         if (loanOf.invoke(null, o) != null) return;
      }
      best.F youngest = null;
      for (Object o : club.kc()) {
         best.F p = (best.F) o;
         if (!p.gl() && (youngest == null || p.getIdade() < youngest.getIdade())) youngest = p;
      }
      Object regular = role.getEnumConstants()[1];
      if (youngest != null && lend.invoke(null, youngest, satellite, regular) == null) SATELLITE_ACTIONS++;
   }

   /** The formation the human picks in the line-up screen (best.aq.rB[5], "4-4-2"). */
   static final int FORMATION = 5;

   static Object call(Object o, String name, Object... args) throws Exception {
      for (Class<?> k = o.getClass(); k != null; k = k.getSuperclass()) {
         for (Method m : k.getDeclaredMethods()) {
            if (m.getName().equals(name) && m.getParameterCount() == args.length) {
               m.setAccessible(true);
               try {
                  return m.invoke(o, args);
               } catch (InvocationTargetException e) {
                  throw (Exception) e.getCause();
               }
            }
         }
      }
      throw new NoSuchMethodException(o.getClass().getName() + "." + name);
   }

   /** Cancels the java.util.Timer a dialog keeps in that field (the next step runs from here instead). */
   static void cancel(Object dialog, String field) throws Exception {
      Object t = EngineHarness.get(dialog, field);
      if (t != null) ((java.util.Timer) t).cancel();
   }

   /**
    * a.bz, the line-up screen: the formation from the combo (bz.cU picks the best players for it),
    * then "confirm" (bz.oh: eleven players, W(true), oi()). oi() ends disposing the dialog, which
    * does not exist here, and calling f.ap(), which the harness does itself.
    */
   public static void lineUp(best.ah club, best.I match) throws Exception {
      a.bz screen = new a.bz(null, club, match, false);
      // the 4-4-2, or else the first formation of the combo that fills the eleven places
      boolean full = false;
      for (int formation : formationsToTry()) {
         call(screen, "cU", formation);
         if (starters(screen) == 11) {
            full = true;
            break;
         }
      }
      if (!full) {
         StringBuilder why = new StringBuilder();
         for (Object o : (ArrayList<?>) EngineHarness.get(screen, "AF")) {
            best.F p = (best.F) o;
            why.append(" ").append(p.getPosicao()).append(":").append(p.getNome());
         }
         throw new IllegalStateException("no formation fills eleven places for " + club.getNome() + ", day " + c.a.SR.J()
            + ", squad " + club.kc().size() + ", available" + why);
      }
      try {
         call(screen, "oh");
      } catch (NullPointerException e) {
         // this.ub.dispose(): the screen has no dialog
      }
      if (!club.kf()) throw new IllegalStateException("line-up of " + club.getNome() + " not confirmed");
   }

   static List<Integer> formationsToTry() {
      List<Integer> order = new ArrayList<>();
      order.add(FORMATION);
      for (int k = 2; k < best.aq.sJ.length; k++) {
         if (k != FORMATION) order.add(k);
      }
      return order;
   }

   /** The players in the eleven places of the line-up screen (bz.oh counts slots 1 to 25). */
   static int starters(a.bz screen) throws Exception {
      ArrayList<?> slots = (ArrayList<?>) EngineHarness.get(screen, "AE");
      int n = 0;
      for (int i = 1; i <= 25; i++) {
         if (((components.ao) slots.get(i)).x() != null) n++;
      }
      return n;
   }

   /** How far ahead the human renews a contract about to end. */
   static final long RENEWAL_HORIZON = 30L * 86_400_000L;

   /**
    * a.fy, the new contract, for each player whose contract ends within RENEWAL_HORIZON (a player
    * of a human club with an ended contract cannot play, F.a(I, boolean)): one year ("1 ano") at the
    * current salary, at most the limit rF() less one; if the player refuses, the human accepts the
    * salary he asks (rE() with "Sim"). dq() ends disposing the dialog, which does not exist here.
    */
   public static void renewContracts(best.ah club) throws Exception {
      long now = c.a.SR.bb().getTimeInMillis();
      for (Object o : new ArrayList<>(club.kc())) {
         best.F p = (best.F) o;
         if ((Long) EngineHarness.get(p, "eJ") >= now + RENEWAL_HORIZON) continue;
         a.fy screen = new a.fy(null, p);
         ((javax.swing.JComboBox<?>) EngineHarness.get(screen, "Bf")).setSelectedIndex(1);
         int limit = (Integer) call(screen, "rF");
         int salary = Math.min(p.fj(), limit - 1);
         if (salary <= 0) continue;
         int agreed = salary;
         if (!(Boolean) call(screen, "dr", salary)) {
            agreed = Math.min((Integer) EngineHarness.get(screen, "IC"), limit);
         }
         try {
            call(screen, "dq", agreed);
         } catch (NullPointerException e) {
            // this.ub.dispose(): the screen has no dialog
         }
         RENEWALS++;
      }
   }

   static int RENEWALS = 0;

   /**
    * a.gO, job offers: a coach without a club takes the first (the row the dialog selects when it
    * opens, then gO.rT(), which ends disposing the dialog); with a club he turns them down.
    */
   public static void jobOffers(ArrayList<?> offers, best.al coach, int kind) {
      try {
         if (kind != 0 || coach.fg() != null || offers.isEmpty()) return;
         a.gO screen = new a.gO(null, offers, coach, kind);
         try {
            call(screen, "rT");
         } catch (NullPointerException e) {
            // this.ub.dispose(): the screen has no dialog
         }
         JOBS_TAKEN++;
      } catch (RuntimeException e) {
         throw e;
      } catch (Exception e) {
         throw new RuntimeException(e);
      }
   }

   static int JOBS_TAKEN = 0;

   /** a.cD, substitutions: opened and closed with no change (cD.pb without the dispose: pc()). */
   public static void substitutions(components.v row, boolean halfTime) {
      try {
         a.cD screen = new a.cD(null, row, halfTime);
         call(screen, "pc");
      } catch (RuntimeException e) {
         throw e;
      } catch (Exception e) {
         throw new RuntimeException(e);
      }
   }

   /** a.gh, the penalty taker: closed without a choice, as the game's automatic mode does; the engine's taker shoots. */
   public static void chooseTaker(components.v row, best.A event) {
   }

   /** a.gb, the penalty: its timer steps in order (gc py(), ge rJ(), gf nO(), gg rK() the dispose). */
   public static void penalty(components.v row, best.A event, a.dV round) {
      try {
         a.gb screen = new a.gb(null, row, event, event.eo(), round);
         cancel(screen, "zC");
         screen.py();
         cancel(screen, "zC");
         call(screen, "rJ");
         cancel(screen, "zC");
         call(screen, "nO");
         cancel(screen, "zC");
      } catch (RuntimeException e) {
         throw e;
      } catch (Exception e) {
         throw new RuntimeException(e);
      }
   }

   /** a.C, the shoot-out order: confirmed as listed (E: dV.a(dF, vp)). */
   public static void shootoutOrder(best.I match, best.ah club) {
      try {
         a.C screen = new a.C(null, club, match);
         a.dV.a((Integer) EngineHarness.get(screen, "dF"), (ArrayList) EngineHarness.get(screen, "vp"));
      } catch (RuntimeException e) {
         throw e;
      } catch (Exception e) {
         throw new RuntimeException(e);
      }
   }

   /** a.bg, the shoot-out: its timer steps (bj nL(), bk nM()) until there is a winner, then "continue". */
   public static void shootout(best.I match) {
      try {
         a.bg screen = new a.bg(null, match);
         cancel(screen, "zC");
         while ((Integer) EngineHarness.get(screen, "zI") == -1) {
            screen.nL();
            cancel(screen, "zC");
            call(screen, "nM");
            cancel(screen, "zC");
         }
      } catch (RuntimeException e) {
         throw e;
      } catch (Exception e) {
         throw new RuntimeException(e);
      }
   }
}
