// @generated automatically by Diesel CLI.

diesel::table! {
    sensor (id) {
        id -> Integer,
        mac_address -> Text,
        name -> Text,
        first_seen -> Timestamp,
    }
}

diesel::table! {
    sensor_data (id) {
        id -> Integer,
        sensor_id -> Integer,
        co2 -> Float,
        temperature -> Float,
        humidity -> Float,
        lumen -> Float,
        reading_time -> Timestamp,
    }
}

diesel::joinable!(sensor_data -> sensor (sensor_id));

diesel::allow_tables_to_appear_in_same_query!(
    sensor,
    sensor_data,
);
