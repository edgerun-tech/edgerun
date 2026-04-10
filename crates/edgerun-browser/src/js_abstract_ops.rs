//! ECMAScript Abstract Operations — generated from ECMA-262.
//! DO NOT EDIT. Regenerate with: scripts/generate_browser.py

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbstractOpId {
    Unspecified = 0,
    Stringindexof = 1,
    Stringlastindexof = 2,
    Numberbitwiseop = 3,
    Binaryand = 4,
    Binaryor = 5,
    Binaryxor = 6,
    Bigintbitwiseop = 7,
    Normalcompletion = 8,
    Throwcompletion = 9,
    Returncompletion = 10,
    Updateempty = 11,
    Ispropertyreference = 12,
    Isunresolvablereference = 13,
    Issuperreference = 14,
    Isprivatereference = 15,
    Getvalue = 16,
    Putvalue = 17,
    Getthisvalue = 18,
    Initializereferencedbinding = 19,
    Makeprivatereference = 20,
    Isaccessordescriptor = 21,
    Isdatadescriptor = 22,
    Isgenericdescriptor = 23,
    Frompropertydescriptor = 24,
    Topropertydescriptor = 25,
    Completepropertydescriptor = 26,
    Createbytedatablock = 27,
    Createsharedbytedatablock = 28,
    Copydatablockbytes = 29,
    Toprimitive = 30,
    Ordinarytoprimitive = 31,
    Toboolean = 32,
    Tonumeric = 33,
    Tonumber = 34,
    Stringtonumber = 35,
    Roundmvresult = 36,
    Tointegerorinfinity = 37,
    Toint32 = 38,
    Touint32 = 39,
    Toint16 = 40,
    Touint16 = 41,
    Toint8 = 42,
    Touint8 = 43,
    Touint8clamp = 44,
    Tobigint = 45,
    Stringtobigint = 46,
    Tobigint64 = 47,
    Tobiguint64 = 48,
    Tostring = 49,
    Toobject = 50,
    Topropertykey = 51,
    Tolength = 52,
    Canonicalnumericindexstring = 53,
    Toindex = 54,
    Requireobjectcoercible = 55,
    Isarray = 56,
    Iscallable = 57,
    Isconstructor = 58,
    Isextensible = 59,
    Isregexp = 60,
    Sametype = 61,
    Samevalue = 62,
    Samevaluezero = 63,
    Samevaluenonnumber = 64,
    Islessthan = 65,
    Islooselyequal = 66,
    Isstrictlyequal = 67,
    Makebasicobject = 68,
    Get = 69,
    Getv = 70,
    Set = 71,
    Createdataproperty = 72,
    Createdatapropertyorthrow = 73,
    Createnonenumerabledatapropertyorthrow = 74,
    Definepropertyorthrow = 75,
    Deletepropertyorthrow = 76,
    Getmethod = 77,
    Hasproperty = 78,
    Hasownproperty = 79,
    Call = 80,
    Construct = 81,
    Setintegritylevel = 82,
    Testintegritylevel = 83,
    Createarrayfromlist = 84,
    Lengthofarraylike = 85,
    Createlistfromarraylike = 86,
    Invoke = 87,
    Ordinaryhasinstance = 88,
    Speciesconstructor = 89,
    Enumerableownproperties = 90,
    Getfunctionrealm = 91,
    Copydataproperties = 92,
    Privateelementfind = 93,
    Privatefieldadd = 94,
    Privatemethodoraccessoradd = 95,
    Hostensurecanaddprivateelement = 96,
    Privateget = 97,
    Privateset = 98,
    Definefield = 99,
    Initializeinstanceelements = 100,
    Addvaluetokeyedgroup = 101,
    Groupby = 102,
    Getoptionsobject = 103,
    Setterthatignoresprototypeproperties = 104,
    Getiteratordirect = 105,
    Getiteratorfrommethod = 106,
    Getiterator = 107,
    Getiteratorflattenable = 108,
    Iteratornext = 109,
    Iteratorcomplete = 110,
    Iteratorvalue = 111,
    Iteratorstep = 112,
    Iteratorstepvalue = 113,
    Iteratorclose = 114,
    Iteratorcloseall = 115,
    Ifabruptcloseiterator = 116,
    Asynciteratorclose = 117,
    Ifabruptcloseasynciterator = 118,
    Createiteratorresultobject = 119,
    Createlistiteratorrecord = 120,
    Iteratortolist = 121,
}

#[derive(Debug, Clone)]
pub struct AbstractOpDef {
    pub id: AbstractOpId,
    pub name: &'static str,
    pub params: &'static [&'static str],
}

pub struct AbstractOpRegistry;
pub const AOP_STRINGINDEXOF: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Stringindexof,
    name: "StringIndexOf",
    params: &["string", "searchValue", "fromIndex"],
};

pub const AOP_STRINGLASTINDEXOF: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Stringlastindexof,
    name: "StringLastIndexOf",
    params: &["string", "searchValue", "fromIndex"],
};

pub const AOP_NUMBERBITWISEOP: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Numberbitwiseop,
    name: "NumberBitwiseOp",
    params: &["op", "x", "y"],
};

pub const AOP_BINARYAND: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Binaryand,
    name: "BinaryAnd",
    params: &["x", "y"],
};

pub const AOP_BINARYOR: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Binaryor,
    name: "BinaryOr",
    params: &["x", "y"],
};

pub const AOP_BINARYXOR: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Binaryxor,
    name: "BinaryXor",
    params: &["x", "y"],
};

pub const AOP_BIGINTBITWISEOP: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Bigintbitwiseop,
    name: "BigIntBitwiseOp",
    params: &["op", "x", "y"],
};

pub const AOP_NORMALCOMPLETION: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Normalcompletion,
    name: "NormalCompletion",
    params: &["value"],
};

pub const AOP_THROWCOMPLETION: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Throwcompletion,
    name: "ThrowCompletion",
    params: &["value"],
};

pub const AOP_RETURNCOMPLETION: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Returncompletion,
    name: "ReturnCompletion",
    params: &["value"],
};

pub const AOP_UPDATEEMPTY: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Updateempty,
    name: "UpdateEmpty",
    params: &["completionRecord", "value"],
};

pub const AOP_ISPROPERTYREFERENCE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Ispropertyreference,
    name: "IsPropertyReference",
    params: &["V"],
};

pub const AOP_ISUNRESOLVABLEREFERENCE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Isunresolvablereference,
    name: "IsUnresolvableReference",
    params: &["V"],
};

pub const AOP_ISSUPERREFERENCE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Issuperreference,
    name: "IsSuperReference",
    params: &["V"],
};

pub const AOP_ISPRIVATEREFERENCE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Isprivatereference,
    name: "IsPrivateReference",
    params: &["V"],
};

pub const AOP_GETVALUE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Getvalue,
    name: "GetValue",
    params: &["V"],
};

pub const AOP_PUTVALUE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Putvalue,
    name: "PutValue",
    params: &["V", "W"],
};

pub const AOP_GETTHISVALUE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Getthisvalue,
    name: "GetThisValue",
    params: &["V"],
};

pub const AOP_INITIALIZEREFERENCEDBINDING: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Initializereferencedbinding,
    name: "InitializeReferencedBinding",
    params: &["V", "W"],
};

pub const AOP_MAKEPRIVATEREFERENCE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Makeprivatereference,
    name: "MakePrivateReference",
    params: &["baseValue", "privateIdentifier"],
};

pub const AOP_ISACCESSORDESCRIPTOR: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Isaccessordescriptor,
    name: "IsAccessorDescriptor",
    params: &["Desc"],
};

pub const AOP_ISDATADESCRIPTOR: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Isdatadescriptor,
    name: "IsDataDescriptor",
    params: &["Desc"],
};

pub const AOP_ISGENERICDESCRIPTOR: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Isgenericdescriptor,
    name: "IsGenericDescriptor",
    params: &["Desc"],
};

pub const AOP_FROMPROPERTYDESCRIPTOR: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Frompropertydescriptor,
    name: "FromPropertyDescriptor",
    params: &["Desc"],
};

pub const AOP_TOPROPERTYDESCRIPTOR: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Topropertydescriptor,
    name: "ToPropertyDescriptor",
    params: &["Obj"],
};

pub const AOP_COMPLETEPROPERTYDESCRIPTOR: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Completepropertydescriptor,
    name: "CompletePropertyDescriptor",
    params: &["Desc"],
};

pub const AOP_CREATEBYTEDATABLOCK: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Createbytedatablock,
    name: "CreateByteDataBlock",
    params: &["size"],
};

pub const AOP_CREATESHAREDBYTEDATABLOCK: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Createsharedbytedatablock,
    name: "CreateSharedByteDataBlock",
    params: &["size"],
};

pub const AOP_COPYDATABLOCKBYTES: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Copydatablockbytes,
    name: "CopyDataBlockBytes",
    params: &["toBlock", "toIndex", "fromBlock", "fromIndex", "count"],
};

pub const AOP_TOPRIMITIVE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Toprimitive,
    name: "ToPrimitive",
    params: &["input \\[preferredType \\]"],
};

pub const AOP_ORDINARYTOPRIMITIVE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Ordinarytoprimitive,
    name: "OrdinaryToPrimitive",
    params: &["O", "hint"],
};

pub const AOP_TOBOOLEAN: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Toboolean,
    name: "ToBoolean",
    params: &["argument"],
};

pub const AOP_TONUMERIC: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Tonumeric,
    name: "ToNumeric",
    params: &["value"],
};

pub const AOP_TONUMBER: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Tonumber,
    name: "ToNumber",
    params: &["argument"],
};

pub const AOP_STRINGTONUMBER: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Stringtonumber,
    name: "StringToNumber",
    params: &["str"],
};

pub const AOP_ROUNDMVRESULT: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Roundmvresult,
    name: "RoundMVResult",
    params: &["n"],
};

pub const AOP_TOINTEGERORINFINITY: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Tointegerorinfinity,
    name: "ToIntegerOrInfinity",
    params: &["argument"],
};

pub const AOP_TOINT32: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Toint32,
    name: "ToInt32",
    params: &["argument"],
};

pub const AOP_TOUINT32: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Touint32,
    name: "ToUint32",
    params: &["argument"],
};

pub const AOP_TOINT16: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Toint16,
    name: "ToInt16",
    params: &["argument"],
};

pub const AOP_TOUINT16: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Touint16,
    name: "ToUint16",
    params: &["argument"],
};

pub const AOP_TOINT8: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Toint8,
    name: "ToInt8",
    params: &["argument"],
};

pub const AOP_TOUINT8: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Touint8,
    name: "ToUint8",
    params: &["argument"],
};

pub const AOP_TOUINT8CLAMP: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Touint8clamp,
    name: "ToUint8Clamp",
    params: &["argument"],
};

pub const AOP_TOBIGINT: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Tobigint,
    name: "ToBigInt",
    params: &["argument"],
};

pub const AOP_STRINGTOBIGINT: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Stringtobigint,
    name: "StringToBigInt",
    params: &["str"],
};

pub const AOP_TOBIGINT64: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Tobigint64,
    name: "ToBigInt64",
    params: &["argument"],
};

pub const AOP_TOBIGUINT64: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Tobiguint64,
    name: "ToBigUint64",
    params: &["argument"],
};

pub const AOP_TOSTRING: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Tostring,
    name: "ToString",
    params: &["argument"],
};

pub const AOP_TOOBJECT: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Toobject,
    name: "ToObject",
    params: &["argument"],
};

pub const AOP_TOPROPERTYKEY: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Topropertykey,
    name: "ToPropertyKey",
    params: &["argument"],
};

pub const AOP_TOLENGTH: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Tolength,
    name: "ToLength",
    params: &["argument"],
};

pub const AOP_CANONICALNUMERICINDEXSTRING: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Canonicalnumericindexstring,
    name: "CanonicalNumericIndexString",
    params: &["argument"],
};

pub const AOP_TOINDEX: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Toindex,
    name: "ToIndex",
    params: &["value"],
};

pub const AOP_REQUIREOBJECTCOERCIBLE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Requireobjectcoercible,
    name: "RequireObjectCoercible",
    params: &["argument"],
};

pub const AOP_ISARRAY: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Isarray,
    name: "IsArray",
    params: &["argument"],
};

pub const AOP_ISCALLABLE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Iscallable,
    name: "IsCallable",
    params: &["argument"],
};

pub const AOP_ISCONSTRUCTOR: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Isconstructor,
    name: "IsConstructor",
    params: &["argument"],
};

pub const AOP_ISEXTENSIBLE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Isextensible,
    name: "IsExtensible",
    params: &["O"],
};

pub const AOP_ISREGEXP: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Isregexp,
    name: "IsRegExp",
    params: &["argument"],
};

pub const AOP_SAMETYPE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Sametype,
    name: "SameType",
    params: &["x", "y"],
};

pub const AOP_SAMEVALUE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Samevalue,
    name: "SameValue",
    params: &["x", "y"],
};

pub const AOP_SAMEVALUEZERO: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Samevaluezero,
    name: "SameValueZero",
    params: &["x", "y"],
};

pub const AOP_SAMEVALUENONNUMBER: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Samevaluenonnumber,
    name: "SameValueNonNumber",
    params: &["x", "y"],
};

pub const AOP_ISLESSTHAN: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Islessthan,
    name: "IsLessThan",
    params: &["x", "y", "LeftFirst"],
};

pub const AOP_ISLOOSELYEQUAL: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Islooselyequal,
    name: "IsLooselyEqual",
    params: &["x", "y"],
};

pub const AOP_ISSTRICTLYEQUAL: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Isstrictlyequal,
    name: "IsStrictlyEqual",
    params: &["x", "y"],
};

pub const AOP_MAKEBASICOBJECT: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Makebasicobject,
    name: "MakeBasicObject",
    params: &["internalSlotsList"],
};

pub const AOP_GET: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Get,
    name: "Get",
    params: &["O", "P"],
};

pub const AOP_GETV: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Getv,
    name: "GetV",
    params: &["V", "P"],
};

pub const AOP_SET: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Set,
    name: "Set",
    params: &["O", "P", "V", "Throw"],
};

pub const AOP_CREATEDATAPROPERTY: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Createdataproperty,
    name: "CreateDataProperty",
    params: &["O", "P", "V"],
};

pub const AOP_CREATEDATAPROPERTYORTHROW: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Createdatapropertyorthrow,
    name: "CreateDataPropertyOrThrow",
    params: &["O", "P", "V"],
};

pub const AOP_CREATENONENUMERABLEDATAPROPERTYORTHROW: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Createnonenumerabledatapropertyorthrow,
    name: "CreateNonEnumerableDataPropertyOrThrow",
    params: &["O", "P", "V"],
};

pub const AOP_DEFINEPROPERTYORTHROW: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Definepropertyorthrow,
    name: "DefinePropertyOrThrow",
    params: &["O", "P", "desc"],
};

pub const AOP_DELETEPROPERTYORTHROW: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Deletepropertyorthrow,
    name: "DeletePropertyOrThrow",
    params: &["O", "P"],
};

pub const AOP_GETMETHOD: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Getmethod,
    name: "GetMethod",
    params: &["V", "P"],
};

pub const AOP_HASPROPERTY: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Hasproperty,
    name: "HasProperty",
    params: &["O", "P"],
};

pub const AOP_HASOWNPROPERTY: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Hasownproperty,
    name: "HasOwnProperty",
    params: &["O", "P"],
};

pub const AOP_CALL: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Call,
    name: "Call",
    params: &["F", "V \\[argumentsList \\]"],
};

pub const AOP_CONSTRUCT: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Construct,
    name: "Construct",
    params: &["F \\[argumentsList \\[newTarget \\] \\]"],
};

pub const AOP_SETINTEGRITYLEVEL: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Setintegritylevel,
    name: "SetIntegrityLevel",
    params: &["O", "level"],
};

pub const AOP_TESTINTEGRITYLEVEL: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Testintegritylevel,
    name: "TestIntegrityLevel",
    params: &["O", "level"],
};

pub const AOP_CREATEARRAYFROMLIST: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Createarrayfromlist,
    name: "CreateArrayFromList",
    params: &["elements"],
};

pub const AOP_LENGTHOFARRAYLIKE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Lengthofarraylike,
    name: "LengthOfArrayLike",
    params: &["obj"],
};

pub const AOP_CREATELISTFROMARRAYLIKE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Createlistfromarraylike,
    name: "CreateListFromArrayLike",
    params: &["obj \\[validElementTypes \\]"],
};

pub const AOP_INVOKE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Invoke,
    name: "Invoke",
    params: &["V", "P \\[argumentsList \\]"],
};

pub const AOP_ORDINARYHASINSTANCE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Ordinaryhasinstance,
    name: "OrdinaryHasInstance",
    params: &["C", "O"],
};

pub const AOP_SPECIESCONSTRUCTOR: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Speciesconstructor,
    name: "SpeciesConstructor",
    params: &["O", "defaultConstructor"],
};

pub const AOP_ENUMERABLEOWNPROPERTIES: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Enumerableownproperties,
    name: "EnumerableOwnProperties",
    params: &["O", "kind"],
};

pub const AOP_GETFUNCTIONREALM: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Getfunctionrealm,
    name: "GetFunctionRealm",
    params: &["obj"],
};

pub const AOP_COPYDATAPROPERTIES: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Copydataproperties,
    name: "CopyDataProperties",
    params: &["target", "source", "excludedItems"],
};

pub const AOP_PRIVATEELEMENTFIND: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Privateelementfind,
    name: "PrivateElementFind",
    params: &["O", "P"],
};

pub const AOP_PRIVATEFIELDADD: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Privatefieldadd,
    name: "PrivateFieldAdd",
    params: &["O", "P", "value"],
};

pub const AOP_PRIVATEMETHODORACCESSORADD: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Privatemethodoraccessoradd,
    name: "PrivateMethodOrAccessorAdd",
    params: &["O", "method"],
};

pub const AOP_HOSTENSURECANADDPRIVATEELEMENT: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Hostensurecanaddprivateelement,
    name: "HostEnsureCanAddPrivateElement",
    params: &["O"],
};

pub const AOP_PRIVATEGET: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Privateget,
    name: "PrivateGet",
    params: &["O", "P"],
};

pub const AOP_PRIVATESET: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Privateset,
    name: "PrivateSet",
    params: &["O", "P", "value"],
};

pub const AOP_DEFINEFIELD: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Definefield,
    name: "DefineField",
    params: &["receiver", "fieldRecord"],
};

pub const AOP_INITIALIZEINSTANCEELEMENTS: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Initializeinstanceelements,
    name: "InitializeInstanceElements",
    params: &["O", "constructor"],
};

pub const AOP_ADDVALUETOKEYEDGROUP: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Addvaluetokeyedgroup,
    name: "AddValueToKeyedGroup",
    params: &["groups", "key", "value"],
};

pub const AOP_GROUPBY: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Groupby,
    name: "GroupBy",
    params: &["items", "callback", "keyCoercion"],
};

pub const AOP_GETOPTIONSOBJECT: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Getoptionsobject,
    name: "GetOptionsObject",
    params: &["options"],
};

pub const AOP_SETTERTHATIGNORESPROTOTYPEPROPERTIES: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Setterthatignoresprototypeproperties,
    name: "SetterThatIgnoresPrototypeProperties",
    params: &["thisValue", "home", "p", "v"],
};

pub const AOP_GETITERATORDIRECT: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Getiteratordirect,
    name: "GetIteratorDirect",
    params: &["obj"],
};

pub const AOP_GETITERATORFROMMETHOD: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Getiteratorfrommethod,
    name: "GetIteratorFromMethod",
    params: &["obj", "method"],
};

pub const AOP_GETITERATOR: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Getiterator,
    name: "GetIterator",
    params: &["obj", "kind"],
};

pub const AOP_GETITERATORFLATTENABLE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Getiteratorflattenable,
    name: "GetIteratorFlattenable",
    params: &["obj", "primitiveHandling"],
};

pub const AOP_ITERATORNEXT: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Iteratornext,
    name: "IteratorNext",
    params: &["iteratorRecord \\[value \\]"],
};

pub const AOP_ITERATORCOMPLETE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Iteratorcomplete,
    name: "IteratorComplete",
    params: &["iteratorResult"],
};

pub const AOP_ITERATORVALUE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Iteratorvalue,
    name: "IteratorValue",
    params: &["iteratorResult"],
};

pub const AOP_ITERATORSTEP: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Iteratorstep,
    name: "IteratorStep",
    params: &["iteratorRecord"],
};

pub const AOP_ITERATORSTEPVALUE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Iteratorstepvalue,
    name: "IteratorStepValue",
    params: &["iteratorRecord"],
};

pub const AOP_ITERATORCLOSE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Iteratorclose,
    name: "IteratorClose",
    params: &["iteratorRecord", "completion"],
};

pub const AOP_ITERATORCLOSEALL: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Iteratorcloseall,
    name: "IteratorCloseAll",
    params: &["iters", "completion"],
};

pub const AOP_IFABRUPTCLOSEITERATOR: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Ifabruptcloseiterator,
    name: "IfAbruptCloseIterator",
    params: &["value", "iteratorRecord"],
};

pub const AOP_ASYNCITERATORCLOSE: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Asynciteratorclose,
    name: "AsyncIteratorClose",
    params: &["iteratorRecord", "completion"],
};

pub const AOP_IFABRUPTCLOSEASYNCITERATOR: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Ifabruptcloseasynciterator,
    name: "IfAbruptCloseAsyncIterator",
    params: &["value", "iteratorRecord"],
};

pub const AOP_CREATEITERATORRESULTOBJECT: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Createiteratorresultobject,
    name: "CreateIteratorResultObject",
    params: &["value", "done"],
};

pub const AOP_CREATELISTITERATORRECORD: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Createlistiteratorrecord,
    name: "CreateListIteratorRecord",
    params: &["list"],
};

pub const AOP_ITERATORTOLIST: AbstractOpDef = AbstractOpDef {
    id: AbstractOpId::Iteratortolist,
    name: "IteratorToList",
    params: &["iteratorRecord"],
};


impl AbstractOpRegistry {
    pub fn by_name(name: &str) -> Option<&'static AbstractOpDef> {
        match name {
            "StringIndexOf" => Some(&AOP_STRINGINDEXOF),
            "StringLastIndexOf" => Some(&AOP_STRINGLASTINDEXOF),
            "NumberBitwiseOp" => Some(&AOP_NUMBERBITWISEOP),
            "BinaryAnd" => Some(&AOP_BINARYAND),
            "BinaryOr" => Some(&AOP_BINARYOR),
            "BinaryXor" => Some(&AOP_BINARYXOR),
            "BigIntBitwiseOp" => Some(&AOP_BIGINTBITWISEOP),
            "NormalCompletion" => Some(&AOP_NORMALCOMPLETION),
            "ThrowCompletion" => Some(&AOP_THROWCOMPLETION),
            "ReturnCompletion" => Some(&AOP_RETURNCOMPLETION),
            "UpdateEmpty" => Some(&AOP_UPDATEEMPTY),
            "IsPropertyReference" => Some(&AOP_ISPROPERTYREFERENCE),
            "IsUnresolvableReference" => Some(&AOP_ISUNRESOLVABLEREFERENCE),
            "IsSuperReference" => Some(&AOP_ISSUPERREFERENCE),
            "IsPrivateReference" => Some(&AOP_ISPRIVATEREFERENCE),
            "GetValue" => Some(&AOP_GETVALUE),
            "PutValue" => Some(&AOP_PUTVALUE),
            "GetThisValue" => Some(&AOP_GETTHISVALUE),
            "InitializeReferencedBinding" => Some(&AOP_INITIALIZEREFERENCEDBINDING),
            "MakePrivateReference" => Some(&AOP_MAKEPRIVATEREFERENCE),
            "IsAccessorDescriptor" => Some(&AOP_ISACCESSORDESCRIPTOR),
            "IsDataDescriptor" => Some(&AOP_ISDATADESCRIPTOR),
            "IsGenericDescriptor" => Some(&AOP_ISGENERICDESCRIPTOR),
            "FromPropertyDescriptor" => Some(&AOP_FROMPROPERTYDESCRIPTOR),
            "ToPropertyDescriptor" => Some(&AOP_TOPROPERTYDESCRIPTOR),
            "CompletePropertyDescriptor" => Some(&AOP_COMPLETEPROPERTYDESCRIPTOR),
            "CreateByteDataBlock" => Some(&AOP_CREATEBYTEDATABLOCK),
            "CreateSharedByteDataBlock" => Some(&AOP_CREATESHAREDBYTEDATABLOCK),
            "CopyDataBlockBytes" => Some(&AOP_COPYDATABLOCKBYTES),
            "ToPrimitive" => Some(&AOP_TOPRIMITIVE),
            "OrdinaryToPrimitive" => Some(&AOP_ORDINARYTOPRIMITIVE),
            "ToBoolean" => Some(&AOP_TOBOOLEAN),
            "ToNumeric" => Some(&AOP_TONUMERIC),
            "ToNumber" => Some(&AOP_TONUMBER),
            "StringToNumber" => Some(&AOP_STRINGTONUMBER),
            "RoundMVResult" => Some(&AOP_ROUNDMVRESULT),
            "ToIntegerOrInfinity" => Some(&AOP_TOINTEGERORINFINITY),
            "ToInt32" => Some(&AOP_TOINT32),
            "ToUint32" => Some(&AOP_TOUINT32),
            "ToInt16" => Some(&AOP_TOINT16),
            "ToUint16" => Some(&AOP_TOUINT16),
            "ToInt8" => Some(&AOP_TOINT8),
            "ToUint8" => Some(&AOP_TOUINT8),
            "ToUint8Clamp" => Some(&AOP_TOUINT8CLAMP),
            "ToBigInt" => Some(&AOP_TOBIGINT),
            "StringToBigInt" => Some(&AOP_STRINGTOBIGINT),
            "ToBigInt64" => Some(&AOP_TOBIGINT64),
            "ToBigUint64" => Some(&AOP_TOBIGUINT64),
            "ToString" => Some(&AOP_TOSTRING),
            "ToObject" => Some(&AOP_TOOBJECT),
            "ToPropertyKey" => Some(&AOP_TOPROPERTYKEY),
            "ToLength" => Some(&AOP_TOLENGTH),
            "CanonicalNumericIndexString" => Some(&AOP_CANONICALNUMERICINDEXSTRING),
            "ToIndex" => Some(&AOP_TOINDEX),
            "RequireObjectCoercible" => Some(&AOP_REQUIREOBJECTCOERCIBLE),
            "IsArray" => Some(&AOP_ISARRAY),
            "IsCallable" => Some(&AOP_ISCALLABLE),
            "IsConstructor" => Some(&AOP_ISCONSTRUCTOR),
            "IsExtensible" => Some(&AOP_ISEXTENSIBLE),
            "IsRegExp" => Some(&AOP_ISREGEXP),
            "SameType" => Some(&AOP_SAMETYPE),
            "SameValue" => Some(&AOP_SAMEVALUE),
            "SameValueZero" => Some(&AOP_SAMEVALUEZERO),
            "SameValueNonNumber" => Some(&AOP_SAMEVALUENONNUMBER),
            "IsLessThan" => Some(&AOP_ISLESSTHAN),
            "IsLooselyEqual" => Some(&AOP_ISLOOSELYEQUAL),
            "IsStrictlyEqual" => Some(&AOP_ISSTRICTLYEQUAL),
            "MakeBasicObject" => Some(&AOP_MAKEBASICOBJECT),
            "Get" => Some(&AOP_GET),
            "GetV" => Some(&AOP_GETV),
            "Set" => Some(&AOP_SET),
            "CreateDataProperty" => Some(&AOP_CREATEDATAPROPERTY),
            "CreateDataPropertyOrThrow" => Some(&AOP_CREATEDATAPROPERTYORTHROW),
            "CreateNonEnumerableDataPropertyOrThrow" => Some(&AOP_CREATENONENUMERABLEDATAPROPERTYORTHROW),
            "DefinePropertyOrThrow" => Some(&AOP_DEFINEPROPERTYORTHROW),
            "DeletePropertyOrThrow" => Some(&AOP_DELETEPROPERTYORTHROW),
            "GetMethod" => Some(&AOP_GETMETHOD),
            "HasProperty" => Some(&AOP_HASPROPERTY),
            "HasOwnProperty" => Some(&AOP_HASOWNPROPERTY),
            "Call" => Some(&AOP_CALL),
            "Construct" => Some(&AOP_CONSTRUCT),
            "SetIntegrityLevel" => Some(&AOP_SETINTEGRITYLEVEL),
            "TestIntegrityLevel" => Some(&AOP_TESTINTEGRITYLEVEL),
            "CreateArrayFromList" => Some(&AOP_CREATEARRAYFROMLIST),
            "LengthOfArrayLike" => Some(&AOP_LENGTHOFARRAYLIKE),
            "CreateListFromArrayLike" => Some(&AOP_CREATELISTFROMARRAYLIKE),
            "Invoke" => Some(&AOP_INVOKE),
            "OrdinaryHasInstance" => Some(&AOP_ORDINARYHASINSTANCE),
            "SpeciesConstructor" => Some(&AOP_SPECIESCONSTRUCTOR),
            "EnumerableOwnProperties" => Some(&AOP_ENUMERABLEOWNPROPERTIES),
            "GetFunctionRealm" => Some(&AOP_GETFUNCTIONREALM),
            "CopyDataProperties" => Some(&AOP_COPYDATAPROPERTIES),
            "PrivateElementFind" => Some(&AOP_PRIVATEELEMENTFIND),
            "PrivateFieldAdd" => Some(&AOP_PRIVATEFIELDADD),
            "PrivateMethodOrAccessorAdd" => Some(&AOP_PRIVATEMETHODORACCESSORADD),
            "HostEnsureCanAddPrivateElement" => Some(&AOP_HOSTENSURECANADDPRIVATEELEMENT),
            "PrivateGet" => Some(&AOP_PRIVATEGET),
            "PrivateSet" => Some(&AOP_PRIVATESET),
            "DefineField" => Some(&AOP_DEFINEFIELD),
            "InitializeInstanceElements" => Some(&AOP_INITIALIZEINSTANCEELEMENTS),
            "AddValueToKeyedGroup" => Some(&AOP_ADDVALUETOKEYEDGROUP),
            "GroupBy" => Some(&AOP_GROUPBY),
            "GetOptionsObject" => Some(&AOP_GETOPTIONSOBJECT),
            "SetterThatIgnoresPrototypeProperties" => Some(&AOP_SETTERTHATIGNORESPROTOTYPEPROPERTIES),
            "GetIteratorDirect" => Some(&AOP_GETITERATORDIRECT),
            "GetIteratorFromMethod" => Some(&AOP_GETITERATORFROMMETHOD),
            "GetIterator" => Some(&AOP_GETITERATOR),
            "GetIteratorFlattenable" => Some(&AOP_GETITERATORFLATTENABLE),
            "IteratorNext" => Some(&AOP_ITERATORNEXT),
            "IteratorComplete" => Some(&AOP_ITERATORCOMPLETE),
            "IteratorValue" => Some(&AOP_ITERATORVALUE),
            "IteratorStep" => Some(&AOP_ITERATORSTEP),
            "IteratorStepValue" => Some(&AOP_ITERATORSTEPVALUE),
            "IteratorClose" => Some(&AOP_ITERATORCLOSE),
            "IteratorCloseAll" => Some(&AOP_ITERATORCLOSEALL),
            "IfAbruptCloseIterator" => Some(&AOP_IFABRUPTCLOSEITERATOR),
            "AsyncIteratorClose" => Some(&AOP_ASYNCITERATORCLOSE),
            "IfAbruptCloseAsyncIterator" => Some(&AOP_IFABRUPTCLOSEASYNCITERATOR),
            "CreateIteratorResultObject" => Some(&AOP_CREATEITERATORRESULTOBJECT),
            "CreateListIteratorRecord" => Some(&AOP_CREATELISTITERATORRECORD),
            "IteratorToList" => Some(&AOP_ITERATORTOLIST),
            _ => None,
        }
    }
}