# E16–E18

{
  "E16_known_tree": {
    "trainable_parameters": 0,
    "decision": "known edge coefficients are recoverable by linear least squares; no neural parameters required"
  },
  "E17_edge_head": {
    "trainable_parameters": 177,
    "best_val_loss": 0.07930436730384827,
    "epochs": 35,
    "fixed_3_node_test": {
      "accuracy": 0.9833333333333333,
      "precision": 0.9135802469107606,
      "recall": 0.9932885906006937,
      "f1": 0.951768488243794
    }
  },
  "E18_variable_topology": {
    "trainable_parameters": 177,
    "test": {
      "edge_accuracy": 0.9822111111111111,
      "edge_f1": 0.6254903422648518,
      "exact_topology": 0.7566666666666667
    },
    "decision": "shared pairwise edge head generalizes across 2-5 nodes in the synthetic pilot"
  }
}