create table sensor (
    id INTEGER PRIMARY KEY NOT NULL,
    mac_address VARCHAR NOT NULL,
    name  VARCHAR NOT NULL,
    first_seen TIMESTAMP NOT NULL,
    UNIQUE (mac_address)
);

create table sensor_data (
    id INTEGER PRIMARY KEY NOT NULL,
    sensor_id INTEGER NOT NULL,
    co2 REAL NOT NULL,
    temperature REAL NOT NULL,
    humidity REAL NOT NULL,
    lumen REAL NOT NULL,
    reading_time TIMESTAMP NOT NULL,
    FOREIGN KEY(sensor_id) REFERENCES sensor(id)
);

create index sensor_data_sensor_id_reading_time
    on sensor_data (sensor_id, reading_time);
