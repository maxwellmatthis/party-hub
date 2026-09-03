-- Secret Santa assignments allow duplicate recipients for manual author edits.
-- Run this once for databases created before version 0.3.0.

CREATE TABLE secret_santa_assignments_new (
    party_id TEXT NOT NULL,
    giver_id TEXT NOT NULL,
    receiver_id TEXT NOT NULL,
    PRIMARY KEY (party_id, giver_id),
    FOREIGN KEY (party_id) REFERENCES parties(id),
    FOREIGN KEY (giver_id) REFERENCES guests(id),
    FOREIGN KEY (receiver_id) REFERENCES guests(id)
);

INSERT INTO secret_santa_assignments_new (party_id, giver_id, receiver_id)
    SELECT party_id, giver_id, receiver_id
    FROM secret_santa_assignments;

DROP TABLE secret_santa_assignments;
ALTER TABLE secret_santa_assignments_new RENAME TO secret_santa_assignments;