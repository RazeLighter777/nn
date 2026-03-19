-- Your SQL goes here
CREATE TABLE `service`(
	`id` INTEGER NOT NULL PRIMARY KEY,
	`nic_target_id` INTEGER NOT NULL,
	`port` INTEGER NOT NULL,
	`ip_proto_number` INTEGER NOT NULL
);

