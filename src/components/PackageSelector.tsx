import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { PackagesConfig, PackageSelection, PhpPackage, MySQLPackage, PhpMyAdminPackage } from "../types/services";

interface PackageSelectorProps {
  onSelectionChange: (selection: PackageSelection) => void;
  initialSelection?: PackageSelection;
}

export function PackageSelector({ onSelectionChange, initialSelection }: PackageSelectorProps) {
  const [packages, setPackages] = useState<PackagesConfig | null>(null);
  const [loading, setLoading] = useState(true);
  const [selection, setSelection] = useState<PackageSelection>(
    initialSelection || {
      php: "php-8.4",
      mysql: "mysql-8.4",
      phpmyadmin: "phpmyadmin-5.2",
    }
  );

  useEffect(() => {
    loadPackages();
  }, []);

  useEffect(() => {
    if (packages) {
      onSelectionChange(selection);
    }
  }, [selection, packages]);

  const loadPackages = async () => {
    try {
      const data = await invoke<PackagesConfig>("get_available_packages_cmd");
      setPackages(data);
    } catch (err) {
      console.error("Failed to load packages:", err);
    } finally {
      setLoading(false);
    }
  };

  const handlePhpChange = (value: string) => {
    setSelection({ ...selection, php: value });
  };

  const handleMySQLChange = (value: string) => {
    setSelection({ ...selection, mysql: value });
  };

  const handlePhpMyAdminChange = (value: string) => {
    setSelection({ ...selection, phpmyadmin: value });
  };

  if (loading) {
    return <div className="package-selector-loading">Loading available packages...</div>;
  }

  if (!packages) {
    return <div className="package-selector-error">Failed to load available packages</div>;
  }

  return (
    <div className="package-selector">
      <div className="package-group">
        <label className="package-label">
          <span>PHP Version</span>
          <span className="package-label-hint">Required for running PHP applications</span>
        </label>
        <select
          value={selection.php}
          onChange={(e) => handlePhpChange(e.target.value)}
          className="package-select"
        >
          {packages.php.map((pkg: PhpPackage) => (
            <option key={pkg.id} value={pkg.id}>
              {pkg.display_name}
              {pkg.eol && " (EOL)"}
              {pkg.recommended && " (Recommended)"}
            </option>
          ))}
        </select>
      </div>

      <div className="package-group">
        <label className="package-label">
          <span>MySQL Version</span>
          <span className="package-label-hint">Database server for your applications</span>
        </label>
        <select
          value={selection.mysql}
          onChange={(e) => handleMySQLChange(e.target.value)}
          className="package-select"
        >
          {packages.mysql.map((pkg: MySQLPackage) => (
            <option key={pkg.id} value={pkg.id}>
              {pkg.display_name}
              {pkg.lts && " (LTS)"}
              {pkg.recommended && " (Recommended)"}
            </option>
          ))}
        </select>
      </div>

      <div className="package-group">
        <label className="package-label">
          <span>phpMyAdmin Version</span>
          <span className="package-label-hint">Web-based database administration tool</span>
        </label>
        <select
          value={selection.phpmyadmin}
          onChange={(e) => handlePhpMyAdminChange(e.target.value)}
          className="package-select"
        >
          {packages.phpmyadmin.map((pkg: PhpMyAdminPackage) => (
            <option key={pkg.id} value={pkg.id}>
              {pkg.display_name}
              {pkg.recommended && " (Recommended)"}
            </option>
          ))}
        </select>
      </div>

      <div className="package-info">
        <p className="package-info-text">
          <strong>Recommended:</strong> PHP 8.5, MySQL 8.4 (LTS), phpMyAdmin 5.2
        </p>
        <p className="package-info-text">
          <strong>Note:</strong> EOL versions may have security vulnerabilities but are provided for legacy application compatibility.
        </p>
      </div>
    </div>
  );
}
