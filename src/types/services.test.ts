import { describe, it, expect, afterEach } from "vitest";
import { dbKeyFor, getDatabaseDisplayName, setActiveDbEngine } from "./services";

describe("database engine helpers", () => {
  afterEach(() => setActiveDbEngine("mysql"));

  it("fixes the engine on Linux and macOS", () => {
    expect(dbKeyFor("linux", "mysql")).toBe("mariadb");
    expect(dbKeyFor("darwin", "mariadb")).toBe("mysql");
    expect(getDatabaseDisplayName("linux")).toBe("MariaDB");
    expect(getDatabaseDisplayName("darwin", "mariadb")).toBe("MySQL");
  });

  it("follows the user's choice on Windows, defaulting to MySQL", () => {
    expect(dbKeyFor("windows")).toBe("mysql");
    expect(getDatabaseDisplayName("windows")).toBe("MySQL");
    setActiveDbEngine("mariadb");
    expect(dbKeyFor("windows")).toBe("mariadb");
    expect(getDatabaseDisplayName("windows")).toBe("MariaDB");
    expect(getDatabaseDisplayName("windows", "mysql")).toBe("MySQL");
  });

  it("treats unknown or missing values as MySQL", () => {
    setActiveDbEngine(undefined);
    expect(dbKeyFor("windows")).toBe("mysql");
  });
});
