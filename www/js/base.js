function handleNotificationMessage(message){

    var title;
    if(message.title){
        title = message.title;
    }else if(message.notification && message.notification.title){
        title = message.notification.title;
    }else if(message.aps && message.aps.alert && message.aps.alert.title){
        title = message.aps.alert.title;
    }

    var body;
    if(message.body){
        body = message.body;
    } else if(message.notification && message.notification.body){
        body = message.notification.body;
    } else if(message.aps && message.aps.alert && message.aps.alert.body){
        body = message.aps.alert.body;
    }

    var msg = "Notification message received";
    if(message.tap){
        msg += " (tapped in " + message.tap + ")";
    }
    if(title){
        msg += '; title='+title;
    }
    if(body){
        msg += '; body='+body;
    }
    msg  += ": "+ JSON.stringify(message);
    consoleX(msg, 'FCM');
};

function handleDataMessage(message){
    consoleX("Data message received: " + JSON.stringify(message), 'FCM');
};

// Notifications
function checkNotificationPermission(requested){
    FirebasePlugin.hasPermission(function(hasPermission) {
        if(hasPermission) {
            consoleX("Remote notifications permission granted", 'FCM');
            // Granted
            getToken();
        } else if(!requested) {
            // Request permission
            consoleX("Requesting remote notifications permission", 'FCM');
            FirebasePlugin.grantPermission(checkNotificationPermission.bind(this, true));
        } else {
            // Denied
            consoleX("Notifications won't be shown as permission is denied", 'FCM');
        }
    });
};

function getID(){
    FirebasePlugin.getId(function(id){
        consoleX("Got FCM ID: " + id)
    }, function(error) {
        consoleX("Failed to get FCM ID", error);
    });
};

function getToken(){
    FirebasePlugin.getToken(function(token){
        consoleX("Got FCM token: " + token, 'FCM');
		tokenNOW = token;
    }, function(error) {
        consoleX("Failed to get FCM token", 'FCM');
    });
};

function api(url, func, method, givenObj, timeout) {
	
	var xhr = new XMLHttpRequest();		
	
	if(!method) method = 'GET';
	if(!timeout) timeout = 2000;
	
	consoleX(""+method+" Request called to " + url, 'XMLHttpRequest');
	
	xhr.onload = function () {
		if (xhr.readyState === 4 && (xhr.status >= 200 && xhr.status < 300)) {
			
			var data = {};
			if(xhr.responseText) {
				data = JSON.parse(xhr.responseText);
			}
			
			consoleX("Status: "+ xhr.status +"! Success while requesting " + url, 'XMLHttpRequest');
			consoleX("Data: "+ JSON.stringify(data), 'XMLHttpRequest');
			func(true, data, xhr.status);
			
		} else {
			
			var data = {};
			if(xhr.responseText) {
				data = JSON.parse(xhr.responseText);
			}
			
			func(false, data, xhr.status);
			consoleX("Status: "+ xhr.status +"! Error while requesting " + url, 'XMLHttpRequest');
		}
	};
	
	xhr.onerror  = function (error) {
		func(false, {}, xhr.status);
		consoleX("Error while requesting " + url, 'XMLHttpRequest');
		consoleX("Error: " + error.code + " " + error.message, 'XMLHttpRequest');
	};

	if(url.substring(0,4) == 'http') {
		xhr.open(method, url);
	};
	xhr.timeout = timeout;
	xhr.setRequestHeader("Content-Type", "application/json");
	xhr.send((givenObj ? JSON.stringify(givenObj) : ''));
	
};

function papi(url, method, givenObj, timeout) {
    
    return new Promise((resolve, reject) => {
        
        api(url, function(result, data,xhr) {
            if(result) {
                resolve(data, xhr)
            } else {
              reject(data, xhr);
            };
        }, method, givenObj, timeout);
        
    });
    
};

function httpGetAsync(theUrl, callback) {
	var xmlHttp = new XMLHttpRequest();
	xmlHttp.onreadystatechange = function() { 
		addToLog(xmlHttp.readyState);
		addToLog(xmlHttp.status);
		if (xmlHttp.readyState == 4 && xmlHttp.status == 200)
			callback(xmlHttp.responseText);
	}
	xmlHttp.open("GET", theUrl, true); // true for asynchronous 
	xmlHttp.send(null);
};

function on(container, event, selector, handler) {
	container.addEventListener(event, function(e){
		if (e.target && e.target.matches(selector)) {
			handler(e.target);
		} else if(e.target.parentNode && e.target.parentNode.matches(selector)) {
			handler(e.target.parentNode);
		} else if(e.target.parentNode && e.target.parentNode.parentNode && e.target.parentNode.parentNode.matches(selector)) {
			handler(e.target.parentNode.parentNode);
		};
		e.stopPropagation();
	});
};

function Base64EncodeUrl(str){
    return str.replace(/\+/g, '-').replace(/\//g, '_').replace(/\=+$/, '');
};

function Base64DecodeUrl(str){
	str = (str + '===').slice(0, str.length + (str.length % 4));
	return str.replace(/-/g, '+').replace(/_/g, '/');
};

function parseJwt(token) {
	var base64UrlX = token.split('.')[1];
	var base64 = base64UrlX.replace(/-/g, '+').replace(/_/g, '/');
	var jsonPayload = decodeURIComponent(atob(base64).split('').map(function(c) {
		return '%' + ('00' + c.charCodeAt(0).toString(16)).slice(-2);
	}).join(''));

	return JSON.parse(jsonPayload);
};

function genReply(request_payload, aid_key_raw, label, fid) {

	var jti = request_payload.jti;
	var eat = request_payload.eat;
	var eid = request_payload.iss;
	var epk = request_payload.aud;
	
	//generate JWT
	var remote_pub = new Uint8Array( FromBase64(eid) );
	var jwt_secret_base64 = ToBase64( axlsign.sharedKey(aid_key_raw.private, remote_pub) );
	consoleX("JWT Sec: " + jwt_secret_base64, 'genReply');

	var data = {
	  "jti": jti,
	  "eat": eat,
	  "iss": ToBase64(aid_key_raw.public),
	  "aud": eid,
	  "epk": epk,
	  "label": label
	};
	
	var header = { 
	  "ecdh": "x25519"
	};

	var jwt = jwt_generate_hs256(header, data, CryptoJS.enc.Base64.parse(jwt_secret_base64));  
	consoleX(jwt, 'genReply');

	//generate MAC
	var remote_pub2 = new Uint8Array( FromBase64(epk) );
	var secret_base64 = ToBase64( axlsign.sharedKey(aid_key_raw.private, remote_pub2) );
	consoleX("HMAC Sec: " + secret_base64, 'genReply');

	var mac = CryptoJS.enc.Base64.stringify( CryptoJS.HmacSHA256(jwt + fid, CryptoJS.enc.Base64.parse(secret_base64)) );
	
	var reply_array_payload = { 
	  "reply": jwt,
	  "fid": fid,
	  "mac": mac,
	  "aid": ToBase64(aid_key_raw.public)
	};

	var reply_array = {
		'payload': reply_array_payload,
		'aid_prv': ToBase64(aid_key_raw.private),
		'aid': ToBase64(aid_key_raw.public),
		'eid': eid
	};
	
	consoleX(JSON.stringify(reply_array), 'genReply');
	
	return reply_array;
};

function ToBase64(u8) {
	return btoa(String.fromCharCode.apply(null, u8));
};

function FromBase64(str) {
	return atob(str).split('').map(function (c) { return c.charCodeAt(0); });
};

const fromHexString = hexString =>
	new Uint8Array(hexString.match(/.{1,2}/g).map(byte => parseInt(byte, 16)));

const toHexString = bytes =>
	bytes.reduce((str, byte) => str + byte.toString(16).padStart(2, '0'), '');

function fromWordArray(wordArray) {
	var words = wordArray.words;
	var sigBytes = wordArray.sigBytes;
	var u8 = new Uint8Array(sigBytes);
	for (var i = 0; i < sigBytes; i++) {
	  var byte = (words[i >>> 2] >>> (24 - (i % 4) * 8)) & 0xff;
	  u8[i]=byte;
	}
	return u8;
};

function base64url(source) {
  // Encode in classical base64
  encodedSource = CryptoJS.enc.Base64.stringify(source);    
  // Remove padding equal characters
  encodedSource = encodedSource.replace(/=+$/, '');    
  // Replace characters according to base64url specifications
  encodedSource = encodedSource.replace(/\+/g, '-');
  encodedSource = encodedSource.replace(/\//g, '_');    
  return encodedSource;
};

function jwt_generate_hs256(header, data, secret) {

	header.alg = "HS256";
	header.typ = "JWT";    

	var encodedHeader = base64url( CryptoJS.enc.Utf8.parse(JSON.stringify(header)) );
	var encodedData = base64url( CryptoJS.enc.Utf8.parse(JSON.stringify(data)) );

	var signature = encodedHeader + "." + encodedData;
	signature = base64url( CryptoJS.HmacSHA256(signature, secret) );

	return encodedHeader + "." + encodedData + "." + signature;
};

function consoleA(object, module) {

};

function consoleX2(text, module) {

};

function consoleX(text, module) {
	consoleApp.insertAdjacentHTML('afterbegin', '<div class="log"><span class="date">' + formatNow() + (module && module.length > 1 ? ' by ' + module : '' ) + '</span> '  +  text + '</div>');
	consoleApp2.insertAdjacentHTML('afterbegin', '<div class="log"><span class="date">' + formatNow() + (module && module.length > 1 ? ' by ' + module : '' ) + '</span> '  +  text + '</div>');
	//consoleApp2.innerHTML = '' + text;
	//console.log(text);
};

function progress(name, code, status) {

};

function consoleQRCode(text) {
	scannedQRs.insertAdjacentHTML('afterbegin', '<div class="log">'  +  text + '</div>');
};

function throttle(func, limit) {
  let lastFunc;
  let lastRan;
  return function() {
    const context = this;
    const args = arguments;
    if (!lastRan) {
      func.apply(context, args);
      lastRan = Date.now();
    } else {
      clearTimeout(lastFunc);
      lastFunc = setTimeout(function() {
        if ((Date.now() - lastRan) >= limit) {
          func.apply(context, args);
          lastRan = Date.now();
        };
      }, limit - (Date.now() - lastRan));
    };
  };
};

function zeroPad(what) {
	return (what < 10 ? '0' + what : what);
};

function formatNow() {
  var now = new Date();
  return (
    zeroPad(now.getHours()) +
    ":" +
    zeroPad(now.getMinutes()) +
    ":" +
    zeroPad(now.getSeconds()) +
    "." +
    now.getMilliseconds()
  );
};

function formatWhen() {
  var now = new Date();
  return (
	zeroPad(now.getFullYear()) +
    " " +
    zeroPad(now.getDate()) +
    "/" +
	zeroPad(now.getMonth()+1) +
    " " +
    zeroPad(now.getHours()) +
    ":" +
    zeroPad(now.getMinutes()) +
    ":" +
    zeroPad(now.getSeconds())
  );
};

// TO Remove
function refreshQRCodes() {
	
	scannedQRs.innerHTML = '';
	if(!db) return false;
	
	db.transaction(function(tx) {
		tx.executeSql('SELECT * FROM Codes', [], function(tx, rs) {
		var len = rs.rows.length, i, html = '';
		for (i = 0; i < len; i++) {
			consoleQRCode(rs.rows.item(i).code);
		}		
	}, function(tx, error) {
	  consoleX('SELECT error: ' + error.message, 'SQLite');
	});

  });
  
  db.transaction(function(tx) {
		tx.executeSql('SELECT * FROM Settings', [], function(tx, rs) {
		var len = rs.rows.length, i, html = '';
		for (i = 0; i < len; i++) {
			consoleQRCode(rs.rows.item(i).name + ' ' + rs.rows.item(i).value);
		}		
	}, function(tx, error) {
	  consoleX('SELECT error: ' + error.message, 'SQLite');
	});
  });
  
};

//key source type
const STOREDKEY_TYPE_ASYMMETRIC		= (1<<15);		//set for asymmetric keys type, clear for symmetric
const STOREDKEY_TYPE_ATTESTED		= (1<<14);		//set for private key/symmetric key never exposed/shown (generated internally only)

const STOREDKEY_KEYS_MASK		= 0x00FF;

// belows valid only when STOREDKEY_TYPE_ASYMMETRIC is set
const STOREDKEY_TYPE_PRIVATEKEY		= (1<<13);		//set if privatekey is present
const STOREDKEY_TYPE_PUBLICKEY		= (1<<12);		//set if publickey is present (first public, then private key )
const STOREDKEY_TYPE_ECDH		= (1<<11);		//set if key valid for ECDH operation (with private key)
const STOREDKEY_TYPE_ECDSA		= (1<<10);		//valid (for ECDSA and EdDSA) ExDSA operation (with private key)
const STOREDKEY_TYPE_CERT		= (1<<9);		//set with publickey means we have certificate

// belows valid only when STOREDKEY_TYPE_ASYMMETRIC is NOT set (symmetric)

// key types 
const STOREDKEY_GENERIC_DER		= 1;		//use only to store X509: with ASYMMETRI+CERT: +PRIVATE=X509_private_key, +PUBLIC=X509_cert,   
const STOREDKEY_ECC_SECP256R1		= 10;		//NIST P-256 (prime256v1)
const STOREDKEY_ECC_SECP384R1		= 11;		//NIST P-384
const STOREDKEY_ECC_SECP521R1		= 12;		//NIST P-521
const STOREDKEY_ECC_SECP256K1		= 13;
const STOREDKEY_ECC_CURVE25519		= 20;
const STOREDKEY_ECC_CURVE448		= 21;
const STOREDKEY_ECC_ED25519		= 22;
const STOREDKEY_ECC_ED448		= 23;

//STOREDKEY_TYPE_ASYMMETRIC | STOREDKEY_TYPE_PRIVATEKEY | STOREDKEY_GENERIC_DER
//STOREDKEY_TYPE_ASYMMETRIC | STOREDKEY_TYPE_CERT | STOREDKEY_GENERIC_DER

// key type where STOREDKEY_TYPE_ASYMMETRIC is NOT set
//#define STOREDKEY_GENERIC 1
const STOREDKEY_HMAC_SHA2_256		= 10;
const STOREDKEY_HMAC_SHA2_384		= 11;
const STOREDKEY_HMAC_SHA2_512		= 12;
const STOREDKEY_HMAC_SHA3_256		= 13;
const STOREDKEY_HMAC_SHA3_384		= 14;
const STOREDKEY_HMAC_SHA3_512		= 15;
const STOREDKEY_AES_128			= 30;
const STOREDKEY_AES_192			= 31;
const STOREDKEY_AES_256			= 32;


function keytype2string(type) {

	var output = '';

		if (type & STOREDKEY_TYPE_ATTESTED) {
			output = output.concat("ATT,");
		}
		if (type & STOREDKEY_TYPE_ASYMMETRIC) {
			
			if (type & STOREDKEY_TYPE_PRIVATEKEY) {
				output = output.concat("PKEY,");
			}

			if (type & STOREDKEY_TYPE_ECDH) {
				output = output.concat("ECDH,");
			}

			if (type & STOREDKEY_TYPE_ECDSA) {
				output = output.concat("ExDSA,");
			}

			if (type & STOREDKEY_TYPE_CERT) {
				output = output.concat("CERT,");
			}

			switch (type & STOREDKEY_KEYS_MASK) {
				case STOREDKEY_GENERIC_DER:
				output = output.concat("GENERIC_DER");
				break;

				case STOREDKEY_ECC_SECP256R1:
				output = output.concat("SECP256R1");
				break;
				case STOREDKEY_ECC_SECP384R1:
				output = output.concat("SECP384R1");
				break;

				case STOREDKEY_ECC_SECP521R1:
				output = output.concat("SECP521R1");
				break;

				case STOREDKEY_ECC_SECP256K1:
				output = output.concat("SECP256K1");
				break;

				case STOREDKEY_ECC_CURVE25519:
				output = output.concat("CURVE25519");
				break;

				case STOREDKEY_ECC_CURVE448:
				output = output.concat("CURVE448");
				break;

				case STOREDKEY_ECC_ED25519:
				output = output.concat("ED25519");
				break;

				case STOREDKEY_ECC_ED448:
				output = output.concat("ED448");
				break;

				default:
				break;
			}
		} else {
			switch (type & STOREDKEY_KEYS_MASK) {
				case STOREDKEY_HMAC_SHA2_256:
				output = output.concat("SHA2-256");
				break;

				case STOREDKEY_HMAC_SHA2_384:
				output = output.concat("SHA2-384");
				break;

				case STOREDKEY_HMAC_SHA2_512:
				output = output.concat("SHA2-512");
				break;
				
				case STOREDKEY_HMAC_SHA3_256:
				output = output.concat("SHA3-256");
				break;

				case STOREDKEY_HMAC_SHA3_384:
				output = output.concat("SHA3-384");
				break;

				case STOREDKEY_HMAC_SHA3_512:
				output = output.concat("SHA3-512");
				break;
				
				case STOREDKEY_AES_128:
				output = output.concat("AES128");
				break;

				case STOREDKEY_AES_192:
				output = output.concat("AES192");
				break;

				case STOREDKEY_AES_256:
				output = output.concat("AES256");
				break;
				
				default:
				output = output.concat("Unknown");
				break;
			}
		}
	
	
	return output;
};
