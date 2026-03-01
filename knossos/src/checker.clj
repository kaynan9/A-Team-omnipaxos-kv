(ns checker
  (:require [clojure.edn :as edn]
            [knossos.model :as model]
            [knossos.competition :as competition]))

(defn read-history
  "Read the EDN history file produced by the test harness."
  [path]
  (edn/read-string (slurp path)))

(defn op-key
  "Extract the key from an operation's :value vector."
  [op]
  (first (:value op)))

(defn transform-op
  "Transform a multi-key operation into a single-register operation
   by stripping the key from the :value vector.

   Multi-key format:
     :read  [:value [\"key\" val-or-nil]]
     :write [:value [\"key\" \"val\"]]
     :cas   [:value [\"key\" \"expected\" \"new\"]]

   Single-register format (Knossos cas-register):
     :read  [:value val-or-nil]
     :write [:value \"val\"]
     :cas   [:value [\"expected\" \"new\"]]"
  [op]
  (let [v (:value op)]
    (case (:f op)
      :read  (assoc op :value (second v))
      :write (assoc op :value (second v))
      :cas   (assoc op :value [(second v) (nth v 2)]))))

(defn group-by-key
  "Group history operations by key. Returns a map of key -> [ops]."
  [history]
  (group-by op-key history))

(defn check-key
  "Run Knossos linearizability check on a single key's history.
   Returns the analysis result."
  [key-name ops]
  (let [history (mapv transform-op ops)
        model   (model/cas-register nil)
        result  (competition/analysis model history)]
    {:key    key-name
     :valid  (:valid? result)
     :result result}))

(defn -main [& args]
  (let [path (or (first args) "history.edn")]
    (println (str "Knossos linearizability checker"))
    (println (str "Reading history from: " path))

    (let [history  (read-history path)
          grouped  (group-by-key history)
          keys     (sort (keys grouped))
          _        (println (str "Found " (count history) " operations across "
                                (count keys) " keys: " (pr-str keys)))
          results  (doall
                     (for [k keys]
                       (do
                         (print (str "  Checking key " (pr-str k) "... "))
                         (flush)
                         (let [r (check-key k (get grouped k))]
                           (println (if (:valid r) "OK" "FAIL"))
                           r))))
          failures (filter #(not (:valid %)) results)]

      (println)
      (if (empty? failures)
        (do
          (println (str "PASS: All " (count keys) " keys are linearizable."))
          (System/exit 0))
        (do
          (println (str "FAIL: " (count failures) " key(s) violated linearizability:"))
          (doseq [f failures]
            (println (str "  - key " (pr-str (:key f)))))
          (System/exit 1))))))


