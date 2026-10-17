import caddy from "./caddy.svg";
import php from "./php.svg";
import mysql from "./mysql.svg";
import mariadb from "./mariadb.svg";
import postgresql from "./postgresql.svg";
import phpmyadmin from "./phpmyadmin.svg";
import adminer from "./adminer.svg";
import ngrok from "./ngrok.svg";

// Keyed by component key (same keys as get_installed_versions / check_existing_components).
export const LOGOS: Record<string, string> = {
  caddy,
  php,
  mysql,
  mariadb,
  postgresql,
  phpmyadmin,
  adminer,
  ngrok,
};
