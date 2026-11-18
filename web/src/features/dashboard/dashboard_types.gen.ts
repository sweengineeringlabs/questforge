// AUTO-GENERATED — do not edit. Regenerate with: mweb generate api dashboard
// Source: dashboard.yaml

export class DashboardData {
  id: string;
  name: string;
  status: string;

  constructor(id: string, name: string, status: string) {
    this.id = id;
    this.name = name;
    this.status = status;
  }

  static builder(): DashboardDataBuilder {
    return new DashboardDataBuilder();
  }
}

class DashboardDataBuilder {
  _id: string = "";
  _name: string = "";
  _status: string = "";

  id(v: string): DashboardDataBuilder { this._id = v; return this; }
  name(v: string): DashboardDataBuilder { this._name = v; return this; }
  status(v: string): DashboardDataBuilder { this._status = v; return this; }

  build(): DashboardData {
    if (this._id === "") throw new Error("DashboardData.id is required");
    if (this._name === "") throw new Error("DashboardData.name is required");
    return new DashboardData(this._id, this._name, this._status);
  }
}

