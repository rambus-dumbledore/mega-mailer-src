create table if not exists "users" (
	"id" bigint not null unique,
	"checking" bool default false,
	"important_tags" text[] default array[]::text[] not null,
	"important_emails" text[] default array[]::text[] not null
);

create table if not exists "mail_accounts" (
	"id" bigint not null,
	"username" text not null,
	"password" text not null,
	foreign key ("id") references "users" ( "id" )
);

create table if not exists "working_hours" (
	"id" bigint not null,
	"start" integer not null,
	"end" integer not null,
	check ("begin" >= 0 and "begin" < 24
		   and "end" >= 0 and "end" < 24
		   and "begin" < "end"),
	foreign key ("id") references "users" ( "id" )
);