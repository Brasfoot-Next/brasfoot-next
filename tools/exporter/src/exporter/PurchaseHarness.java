package exporter;

import java.io.*;
import java.nio.file.*;
import java.util.*;

/**
 * Differential reference for a human club's purchases (crates/simulation/src/human/purchase.rs).
 * Loads human-start.s22 (the fictional career with its human coach at club 1), asks the game's
 * negotiation best.l.a(F, ah, int) about the players of the first other clubs at several prices
 * (purchase-answers.txt: player, buyer id, buyer country, amount, answer, salary asked cM(),
 * counter price cN(); the buyer's country changed for the question to one of each other
 * confederation too), then
 * closes three deals the way a.cz.oL() does (F.a(ah, int, ...) and best.l.a(ah, F)) with the
 * no-arg Randoms on the counter, and saves purchase-after.s22.
 *
 * Usage: PurchaseHarness <fixture dir> <first seed>, user.dir an empty folder (export.sh purchase).
 */
public class PurchaseHarness {
   static final double[] SHARES = {0.5, 0.9, 1.0, 1.25, 1.6, 2.2, 3.0};

   public static void main(String[] args) throws Exception {
      Path dir = Paths.get(args[0]);
      long seed = Long.parseLong(args[1]);
      Files.write(Paths.get(System.getProperty("user.dir"), "bfmod.config"), SimHarness.SYSTEMS_OFF.getBytes());
      new best.M(true);
      best.n.cU();
      EngineHarness.load(dir.resolve("human-start.s22").toString());
      best.ah buyer = c.a.SR.x(1);
      StringBuilder out = new StringBuilder();
      List<best.F> accepted = new ArrayList<>();
      // the human club and one club of each reputation as buyers (the negotiation reads only
      // the buyer's reputation, division, country and cash)
      List<best.ah> buyers = new ArrayList<>();
      buyers.add(buyer);
      for (int rep = 1; rep <= 4; rep++) {
         for (Object o : c.a.SR.P()) {
            best.ah t = (best.ah) o;
            if (t != buyer && t.getReputacao() == rep && !buyers.contains(t)) {
               buyers.add(t);
               break;
            }
         }
      }
      // the same buyers as if from a country of each other confederation (the player's will
      // depends on it): the country is changed for the questions and put back
      List<Integer> foreign = new ArrayList<>();
      Set<Integer> confederations = new HashSet<>();
      confederations.add(buyer.gg());
      for (int id = 1; id < 260; id++) {
         try {
            if (confederations.add(best.ac.valueOf("P" + id).gg())) foreign.add(id);
         } catch (IllegalArgumentException ignored) {
         }
      }
      int clubs = 0;
      for (Object o : c.a.SR.P()) {
         best.ah club = (best.ah) o;
         if (club == buyer || clubs++ >= 5) continue;
         for (Object po : club.kc()) {
            best.F p = (best.F) po;
            for (best.ah b : buyers) {
               if (b == club) continue;
               int home = b.getPais();
               List<Integer> countries = new ArrayList<>();
               countries.add(home);
               countries.addAll(foreign);
               for (int country : countries) {
                  b.setPais(country);
                  for (double share : SHARES) {
                     int amount = (int) Math.round(p.fk() * share);
                     int answer = best.l.a(p, b, amount);
                     out.append(p.getNome()).append('|').append(b.lk()).append('|').append(country).append('|').append(amount).append('|').append(answer).append('|')
                        .append(best.l.cM()).append('|').append(best.l.cN()).append('\n');
                     if (b == buyer && country == home && answer == 1 && share == 3.0 && accepted.size() < 3 && !accepted.contains(p) && p.fk() * 3L < buyer.kb()) {
                        accepted.add(p);
                     }
                  }
               }
               b.setPais(home);
            }
         }
      }
      Files.write(dir.resolve("purchase-answers.txt"), out.toString().getBytes("UTF-8"));
      EngineHarness.resetRandomness(seed);
      for (best.F p : accepted) {
         best.ah seller = p.fg();
         p.a(buyer, (int) Math.round(p.fk() * 3.0), false, false, false);
         best.l.a(seller, p);
      }
      EngineHarness.write(dir.resolve("purchase-after.s22").toString());
      System.out.println(accepted.size() + " purchases");
   }
}
