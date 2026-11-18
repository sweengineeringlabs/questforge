// AUTO-GENERATED — do not edit. Regenerate with: mweb generate api dashboard
// Source: dashboard.yaml

export class AuditLogEntry {
  method: string;
  path: string;
  role: string;
  seq: number;
  status_code: number;

  constructor(method: string, path: string, role: string, seq: number, status_code: number) {
    this.method = method;
    this.path = path;
    this.role = role;
    this.seq = seq;
    this.status_code = status_code;
  }

  static builder(): AuditLogEntryBuilder {
    return new AuditLogEntryBuilder();
  }
}

class AuditLogEntryBuilder {
  _method: string = "";
  _path: string = "";
  _role: string = "";
  _seq: number = 0;
  _status_code: number = 0;

  method(v: string): AuditLogEntryBuilder { this._method = v; return this; }
  path(v: string): AuditLogEntryBuilder { this._path = v; return this; }
  role(v: string): AuditLogEntryBuilder { this._role = v; return this; }
  seq(v: number): AuditLogEntryBuilder { this._seq = v; return this; }
  status_code(v: number): AuditLogEntryBuilder { this._status_code = v; return this; }

  build(): AuditLogEntry {
    if (this._method === "") throw new Error("AuditLogEntry.method is required");
    if (this._path === "") throw new Error("AuditLogEntry.path is required");
    if (this._role === "") throw new Error("AuditLogEntry.role is required");
    if (this._seq === 0) throw new Error("AuditLogEntry.seq is required");
    if (this._status_code === 0) throw new Error("AuditLogEntry.status_code is required");
    return new AuditLogEntry(this._method, this._path, this._role, this._seq, this._status_code);
  }
}

export class BuildRecord {
  id: string;
  project_id: string;
  status: string;
  version: string;

  constructor(id: string, project_id: string, status: string, version: string) {
    this.id = id;
    this.project_id = project_id;
    this.status = status;
    this.version = version;
  }

  static builder(): BuildRecordBuilder {
    return new BuildRecordBuilder();
  }
}

class BuildRecordBuilder {
  _id: string = "";
  _project_id: string = "";
  _status: string = "";
  _version: string = "";

  id(v: string): BuildRecordBuilder { this._id = v; return this; }
  project_id(v: string): BuildRecordBuilder { this._project_id = v; return this; }
  status(v: string): BuildRecordBuilder { this._status = v; return this; }
  version(v: string): BuildRecordBuilder { this._version = v; return this; }

  build(): BuildRecord {
    if (this._id === "") throw new Error("BuildRecord.id is required");
    if (this._project_id === "") throw new Error("BuildRecord.project_id is required");
    if (this._status === "") throw new Error("BuildRecord.status is required");
    if (this._version === "") throw new Error("BuildRecord.version is required");
    return new BuildRecord(this._id, this._project_id, this._status, this._version);
  }
}

export class DeploymentRecord {
  apk_path: string;
  status: string;
  target: string;

  constructor(apk_path: string, status: string, target: string) {
    this.apk_path = apk_path;
    this.status = status;
    this.target = target;
  }

  static builder(): DeploymentRecordBuilder {
    return new DeploymentRecordBuilder();
  }
}

class DeploymentRecordBuilder {
  _apk_path: string = "";
  _status: string = "";
  _target: string = "";

  apk_path(v: string): DeploymentRecordBuilder { this._apk_path = v; return this; }
  status(v: string): DeploymentRecordBuilder { this._status = v; return this; }
  target(v: string): DeploymentRecordBuilder { this._target = v; return this; }

  build(): DeploymentRecord {
    if (this._apk_path === "") throw new Error("DeploymentRecord.apk_path is required");
    if (this._status === "") throw new Error("DeploymentRecord.status is required");
    if (this._target === "") throw new Error("DeploymentRecord.target is required");
    return new DeploymentRecord(this._apk_path, this._status, this._target);
  }
}

export class ProjectRecord {
  id: string;
  name: string;
  owner: string;
  repository: string;

  constructor(id: string, name: string, owner: string, repository: string) {
    this.id = id;
    this.name = name;
    this.owner = owner;
    this.repository = repository;
  }

  static builder(): ProjectRecordBuilder {
    return new ProjectRecordBuilder();
  }
}

class ProjectRecordBuilder {
  _id: string = "";
  _name: string = "";
  _owner: string = "";
  _repository: string = "";

  id(v: string): ProjectRecordBuilder { this._id = v; return this; }
  name(v: string): ProjectRecordBuilder { this._name = v; return this; }
  owner(v: string): ProjectRecordBuilder { this._owner = v; return this; }
  repository(v: string): ProjectRecordBuilder { this._repository = v; return this; }

  build(): ProjectRecord {
    if (this._id === "") throw new Error("ProjectRecord.id is required");
    if (this._name === "") throw new Error("ProjectRecord.name is required");
    return new ProjectRecord(this._id, this._name, this._owner, this._repository);
  }
}

export class VerificationRecord {
  id: string;
  ready: boolean;
  score: number;

  constructor(id: string, ready: boolean, score: number) {
    this.id = id;
    this.ready = ready;
    this.score = score;
  }

  static builder(): VerificationRecordBuilder {
    return new VerificationRecordBuilder();
  }
}

class VerificationRecordBuilder {
  _id: string = "";
  _ready: boolean = false;
  _score: number = 0;

  id(v: string): VerificationRecordBuilder { this._id = v; return this; }
  ready(v: boolean): VerificationRecordBuilder { this._ready = v; return this; }
  score(v: number): VerificationRecordBuilder { this._score = v; return this; }

  build(): VerificationRecord {
    if (this._id === "") throw new Error("VerificationRecord.id is required");
    if (this._score === 0) throw new Error("VerificationRecord.score is required");
    return new VerificationRecord(this._id, this._ready, this._score);
  }
}

