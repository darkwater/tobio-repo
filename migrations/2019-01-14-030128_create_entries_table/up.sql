CREATE TABLE entries (
    id              INTEGER PRIMARY KEY NOT NULL,
    parent          INTEGER NOT NULL,
    entry_type      TEXT NOT NULL,
    key             TEXT NOT NULL,
    label           TEXT NOT NULL,
    url             TEXT,
    extra           TEXT,
    provider_extra  TEXT,

    UNIQUE ( parent, key )
);
