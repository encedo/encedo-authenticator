/*
 * Licensed to the Apache Software Foundation (ASF) under one
 * or more contributor license agreements.  See the NOTICE file
 * distributed with this work for additional information
 * regarding copyright ownership.  The ASF licenses this file
 * to you under the Apache License, Version 2.0 (the
 * "License"); you may not use this file except in compliance
 * with the License.  You may obtain a copy of the License at
 *
 * http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing,
 * software distributed under the License is distributed on an
 * "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
 * KIND, either express or implied.  See the License for the
 * specific language governing permissions and limitations
 * under the License.
 */

// Wait for the deviceready event before using any of Cordova's device APIs.
// See https://cordova.apache.org/docs/en/latest/cordova/events/events.html#deviceready

var db = null;
var bodyDOM = false;
var tokenNOW = 'gne708d423230yned982y0-gxef79e-gh80-023';
var already = false;
var loaded = [];
var actions = [];
var lastPage = false;
var lastPage2 = false;
var lastClass = 'transparent';
var archiveStarted = false;
var devicesStarted = false;
var initAlready = false;
var _blocked = true;
var _blockable = false;
var _production = true;
var _backgroundAllowed = false;
var _version = 0.8872;
var _isFinger = false;
var _session = false;
var _started = false;
var superCounter = 0;

var _settings = {
	firstTimeOpened: 1,
	lockedOnStart: 2,
	preventScreenShot: 2,
	notificationsOn: 2,
	blockOnShake: 2
};

var _settingsNames = {
	firstTimeOpened: ['', 'Application has been opened for the first time.', ''],
	lockedOnStart: ['', 'Authentication with PIN or fingerprint has been <i>activated</i>.', 'Authentication with PIN or fingerprint has been <i>deactivated</i>.'],
	preventScreenShot: ['', 'From now on screenshots of application screen is <i>disabled</i>.', 'Making screenshots in this application is possible now.'],
	notificationsOn: ['', 'Notifications has been switched on.', 'Notifications has been switched off.'],
	blockOnShake: ['', 'From now on shaking your phone will block the app immediately.', 'Blocking application while phone shaking has been disabled.']
};

if(typeof cordova === 'undefined') {
	_production = false;
};

if(!_production) {
	window.addEventListener('load', onDeviceReady);
}

document.addEventListener('deviceready', function(){
	consoleX('CordovaDeviceReady', 'CordovaDeviceReady');
	onDeviceReady();
}, false);

function onDeviceReady() {

    var loadingSplashLogoCounter = 0;

    loadingSplashLogo.onclick = function(){
        loadingSplashLogoCounter++;
        if(loadingSplashLogoCounter > 5) {
            consoleApp2.style.display = 'block';
            loadingDOM.classList.remove('centeredV');
            loadingDOM.classList.remove('loading');
            loadingDOM.classList.remove('page');
            loadingDOM.classList.remove('pageActive');
        };
    };
	
	if(_production) {
		
		// Creating session unique name
		var sess = new Uint8Array(32);
		window.crypto.getRandomValues(sess);
		_session = ToBase64(sess);
		
		consoleX('Session key generated', 'CordovaDeviceReady');
		
		screen.orientation.lock('portrait');
		
		consoleX('Portrait orientation locked', 'CordovaDeviceReady');
	
		cordova.plugins.SecureKeyStore.get(function (res) {
			
		  startWithDatabase(res);
		  
		}, function (error) {
			
			var array = new Uint8Array(32);
			window.crypto.getRandomValues(array);
			var finalise = ToBase64(array);
		
			cordova.plugins.SecureKeyStore.set(function (res) {
				
			  startWithDatabase(finalise);
			  
			}, function (error) {
				showProblem('Problem<br><strong>No access</strong>', 'Unfortunately we cannot grant you access to this app.', 'Sorry', function(){
					_blocked = true;
					blockApp();
				});
			}, "supersecretto", finalise);
		  
		}, "supersecretto");
	
	} else {
		startWithDatabase('blank');
	};

};

function startWithDatabase(db_password) {
	
	if(_production) {
		consoleX('Running cordova-' + cordova.platformId + '@' + cordova.version, 'CordovaDeviceReady');
	} else {
		consoleX('Running without cordova.', 'CordovaDeviceReady');
	};
	
	if(window.sqlitePlugin) {
		
		window.sqlitePlugin.echoTest(function() {
			consoleX('ECHO test OK', 'SQLite');
	    });
		
		var dbname = db_password.substr(0,6);
	
		db = window.sqlitePlugin.openDatabase({
			name: 'encedo_' + dbname,
			location: 'default',
			key: db_password
		});
		
		db.transaction(function(tx) {
			tx.executeSql('CREATE TABLE IF NOT EXISTS Settings (name, value)');
		}, function(error) {
			consoleX('Transaction ERROR on "Settings": ' + error.message, 'SQLite');
		}, function() {
			consoleX('"Settings" database created or was already here.', 'SQLite');
		});
		
		db.transaction(function(tx) {
			tx.executeSql('CREATE TABLE IF NOT EXISTS Devices (pid, eid, aid_prv, aid, iat, datetime, user, email, host)');
		}, function(error) {
			consoleX('Transaction ERROR on "Devices": ' + error.message, 'SQLite');
		}, function() {
			consoleX('"Devices" database created or was already here.', 'SQLite');
			printDevices();
		});
		
		db.transaction(function(tx) {
			tx.executeSql('CREATE TABLE IF NOT EXISTS `Archives` (name, desc, status, date, pid)');
		}, function(error) {
			consoleX('Transaction ERROR on "Archives": ' + error.message, 'SQLite');
		}, function() {
			consoleX('"Archives" database created or was already here.', 'SQLite');
			
			db.transaction(function(tx) {
				tx.executeSql('ALTER TABLE `Archives` ADD COLUMN pid;');
			}, function(error) {
				consoleX('Transaction ERROR on "Archives": ' + error.message, 'SQLite');
			}, function() {
				consoleX('"Archives" database has been altered.', 'SQLite');
			});
		
		});
		
		db.transaction(function(tx) {
			tx.executeSql('CREATE TABLE IF NOT EXISTS Events (eventId, pid, name, type, result)');
		}, function(error) {
			consoleX('Transaction ERROR: ' + error.message, 'SQLite');
		}, function() {
			consoleX('"Events" database created or was already here.', 'SQLite');			
		});

		db.transaction(function(tx) {
            tx.executeSql('SELECT * FROM Settings', [], function(tx, rs) {
                var len = rs.rows.length, i;
                for (i = 0; i < len; i++) {
                    _settings[rs.rows.item(i).name] = rs.rows.item(i).value;
                    consoleX('Start app: Name: '+rs.rows.item(i).name+' with value: ' + rs.rows.item(i).value, 'SQLite');
                }
                prepareHandlers();
                checkInit();
                consoleX('Ready for prepareHandlers.', 'SQLite');
            }, function(tx, error) {
              consoleX('SELECT error from Settings: ' + error.message, 'SQLite');
            });
        });
	
	} else {
		prepareHandlers();
		checkInit();
	};
};
	
function checkInit() {
	
	if(_production) {
		
		Fingerprint.isAvailable(isAvailableSuccess, isAvailableError, { allowBackup: true, disableBackup: false, confirmationRequired: false });

		function isAvailableSuccess(result) {
		  if(result) {
			  consoleX("Fingerprint available", 'FingerprintAuth');
			  consoleX2("Fingerprint available", 'FingerprintAuth');
			  _isFinger = true;
			  initWithSettings();
		  } else {
			  consoleX("Fingerprint NOT available", 'FingerprintAuth');
			  consoleX2("Fingerprint NOT available", 'FingerprintAuth');
			  _isFinger = false;
			  initWithSettings();
		  };
		};

		function isAvailableError(error) {
		  consoleX('Error: ' + error.message, 'FingerprintAuth');
		  consoleX2('Error: ' + error.message, 'FingerprintAuth');
		  _isFinger = false;
		  initWithSettings();
		};
		
		//Register handlers
		FirebasePlugin.onMessageReceived(function(message) {
			try{
				
				consoleX("onMessageReceived", 'FCM');
				consoleX(JSON.stringify(message), 'FCM');
				handleEvent(message);
				
				if(message.messageType === "notification"){
					handleNotificationMessage(message);
				} else {
					handleDataMessage(message);
				};
				
			} catch(e){
				consoleX("Exception in onMessageReceived callback: "+e.message, 'FCM');
			};

		}, function(error) {
			consoleX("Failed receiving FirebasePlugin message", 'FCM');
		});

		FirebasePlugin.onTokenRefresh(function(token){
			consoleX("Token refreshed: " + token, 'FCM')
		}, function(error) {
			consoleX("Failed to refresh token", 'FCM');
		});
		
		checkNotificationPermission(false);
		
	} else {
		
		initWithSettings();
		
	};

};

function initWithSettings() {
	
	if(initAlready) return false;
	
	initAlready = true;
	
	consoleX('Now initWithSettings is playing.', 'UI');
	
	var radioFields = document.querySelectorAll('.settingOption');
	radioFields.forEach(function(item) {
		
		item.addEventListener("click", function(event){
			
			_backgroundAllowed = true;
			var valg = 2;
			
			if(item.checked) {
				valg = 1;
			};
			
			consoleX('Setting changed: ' + item.name + ' with value ' + valg, 'UI');
			if(_settingsNames[item.name] && _settingsNames[item.name][valg]) {
				archive('User setting has been changed', _settingsNames[item.name][valg], 'ok');
			} else {
				archive('User setting has been changed', 'Setting changed: ' + item.name + ' with value ' + valg, 'ok');
			}
			setSetting(item.name, valg);
			_backgroundAllowed = false;

		});
		
		if(_settings[item.name] == 1) {
			item.checked = true;
		} else {
			item.checked = false;
		};
		
	});
	
	if(_production && _isFinger && getSetting('blockOnShake') == 1) {
		
		var onShake = function () {
		    if(superCounter > 10) {
		        changePage('consolelog');
		    } else {
		        setTimeout(function(){
                   _blocked = true;
                   blockApp();
                }, 1);
		    };

		};
		 
		var onError = function () {
		  // Fired when there is an accelerometer error (optional)
		};
		 
		// Start watching for shake gestures and call "onShake"
		// with a shake sensitivity of 40 (optional, default 30)
		shake.startWatch(onShake, 40, onError);
		
	} else {

	};
	
	if(_production) {
		if(getSetting('preventScreenShot') == 1) {
		
			var successCallbackScreen = function(){
				consoleX("The screenshots are not allowed now.", 'ScreenShotPrevention');
			};

			var errorCallbackScreen = function(err){
				consoleX("An error ocurred : " + err, 'ScreenShotPrevention');
			};
		
			window.plugins.preventscreenshot.disable(successCallbackScreen, errorCallbackScreen);
			
		} else {
			
			var successCallbackScreen = function(){
				consoleX("The screenshots are allowed now.", 'ScreenShotPrevention');
			};

			var errorCallbackScreen = function(err){
				consoleX("An error ocurred : " + err, 'ScreenShotPrevention');
			};
		
			window.plugins.preventscreenshot.enable(successCallbackScreen, errorCallbackScreen);
			
		};
	};
	
	if(getSetting('firstTimeOpened') == 1) {
		
		unblockApp(); 
		
		consoleX('First time opened is true right now.', 'UI');
		consoleX2('First time opened is true right now.', 'UI');
		archive('First App opening!', 'Yay! Welcome to the Encedo Family. From now on you can rely on us when it comes to data security. App version is ' + _version, 'ok');
		
		if(_production) {
			startFinalApp('welcome');
		} else {
			startFinalApp('homeNew');

			setTimeout(function(){
				
				showOK(
				'<strong>Do you see this prompt?</strong>', 
				'New device owned by has been successfully connected with this application.', 
				'Paired devices', 
				function(){
					changePage('dashboard');
				}
			);	
			}, 200);
			
			
			setTimeout(function(){
				
				showOK(
				'<strong>Is this visible?</strong>', 
				'New device owned by has been successfully connected with this application.', 
				'Paired devices', 
				function(){
					changePage('dashboard');
				}
			);	
			}, 300);
						
		}
		setSetting('firstTimeOpened', 2);
		
	} else {
		
		cordova.plugins.SecureKeyStore.get(function (res) {
		  if(res < _version) {
			  archive('App has been updated', 'It is great to keep things up to date. Thank you for caring!', 'ok');
			  cordova.plugins.SecureKeyStore.set(function (res) {
			  
			}, function (error) {
				
			}, "version", _version);
		  } else {
			  
		  }
		}, function (error) {
			
			cordova.plugins.SecureKeyStore.set(function (res) {
			  
			}, function (error) {
				
			}, "version", _version);
		  
		}, "version");
		
		consoleX('This is not first time app is being opened.', 'UI');
		consoleX2('This is not first time app is being opened.', 'UI');
		archive('App opened', 'Just a trace of application opening. It is good to keep track of that kind of events. Version of the app seems to be ' + _version, 'ok');
		
		if(_isFinger && getSetting('lockedOnStart') == 1) {
			
			consoleX('Application is blocked and needs to be opened.', 'UI');
			
			_blockable = true;
			_backgroundAllowed = true;
			
			Fingerprint.show({
			  title: 'Fingerprint Authentication',
			  description: "Application is locked. Please Sign on before you go any further."
			}, successCallback, errorCallback);

			function successCallback(){
				
				_blocked = false;
				
				unblockApp();
				
				consoleX("Authentication successful", 'FingerprintAuth');
				
				startFinalApp('homeNew');
				
				_backgroundAllowed = false;
			}

			function errorCallback(error){
			  consoleX("Authentication error" + error.message, 'FingerprintAuth');
			  archive('Invalid authentication', 'Someone opened this app and tried to unlock it. Authentication failed. Beware!', 'lock');
		
			   blockApp();
			   startFinalApp('homeNew');
			  _backgroundAllowed = false;
			}
			
		} else {
			
			consoleX('Application is unlocked by default.', 'UI');
			
			consoleX2('No finger', 'UI');
			_blocked = false;
			_blockable = false;
			
			unblockApp();
			
			startFinalApp('homeNew');
			
		};
		
	};

};

function startFinalApp(startingPage, arg1, arg2) {
	
	if(!_started) {
		
		window.scrollTo(0, 0);
		
		document.body.classList.remove('offline');
		//finalAPP.classList.remove('offline');
		//document.getElementById('loadingDOM').classList.add("loaded");

		consoleX('Application started and authenticated.', 'Core');
		
		_started = true;
	
	};
	
	on(document, 'click', '.makeAction', function(e){
		var rel = e.getAttribute("rel").split('/');
		if(actions[rel[0]]) {
			actions[rel[0]](rel[1], rel[2], rel[3], rel[4]);
		};
	});
	
	on(document, 'click', '.changePage', function(e){
		var rel = e.getAttribute("rel");
		changePage(rel);
	});
	
	on(document, 'click', '.toggle', function(e){
		var rel = e.getAttribute("rel");
		e.classList.toggle("opened");
		document.getElementById(rel).classList.toggle("dead");
	});
	
	on(document, 'click', '.changeTo', function(e){
		var rel = e.getAttribute("rel");
		e.innerHTML = rel;
	});
	
	if(startingPage) {
		setTimeout(function(){ 
			changePage(startingPage, arg1, arg2); 
			checkEventsToHandle();
		}, 3);
	} else {
		setTimeout(function(){ 
			changePage('welcome', arg1, arg2); 
			checkEventsToHandle();
		}, 3);
	};
	
};

function getSetting(name) {
	return _settings[name];
};

function setSetting(name, value) {

	_settings[name] = value;
	
	if(name == 'preventScreenShot') {
		if(value == 1) {
			
			var successCallbackScreen = function(){
				consoleX("The screenshots are NOT allowed now.", 'ScreenShotPrevention');
			};

			var errorCallbackScreen = function(err){
				consoleX("An error ocurred : " + err, 'ScreenShotPrevention');
			};
		
			window.plugins.preventscreenshot.disable(successCallbackScreen, errorCallbackScreen);
		
		
		} else {
			
			var successCallbackScreen = function(){
				consoleX("The screenshots are allowed now.", 'ScreenShotPrevention');
			};

			var errorCallbackScreen = function(err){
				consoleX("An error ocurred : " + err, 'ScreenShotPrevention');
			};
		
			window.plugins.preventscreenshot.enable(successCallbackScreen, errorCallbackScreen);
		}
	};;
	
	if(name == 'lockedOnStart') {
		if(value == 1) {
			_blockable = true;
		} else {
			_blockable = false;
		};
	};
	
	if(name == 'blockOnShake') {
		if(value == 1) {
			var onShake = function () {
                if(superCounter > 10) {
                    changePage('consolelog');
                } else {
                    setTimeout(function(){
                       _blocked = true;
                       blockApp();
                    }, 1);
                };

            };
			 
			var onError = function () {
			  // Fired when there is an accelerometer error (optional)
			};
			 
			// Start watching for shake gestures and call "onShake"
			// with a shake sensitivity of 40 (optional, default 30)
			shake.startWatch(onShake, 40, onError);
		} else {
			shake.startWatch(function(){
				consoleX('Shaked but off', 'ShakeItBaby');
			}, 40, onError);
		};
	};
	
	if(db) {
		db.transaction(function(tx) {
			tx.executeSql('INSERT INTO Settings (name, value) VALUES (?, ?)', [name, value]);
		}, function(error) {
			consoleX('Transaction ERROR: ' + error.message, 'SQLite');
		}, function() {
			consoleX('"Settings" passed with update for '+name+' with value: '+value+'', 'SQLite');
		});
	}
};

function handleEventError(code, resultX, scopeTmp, pid, geoloco) {
	if(code == '401') {
		archive('Critical error', 'It looks like quite error happened.' + geoloco, 'cancel', pid);
		showProblem('<strong>Critical error</strong>', 'It looks like quite error happened. Please try again in a while.', 'Go home', 'homeNew');
	} else if(code == '404') {
		archive('Event expired', 'Requested ('+scopeTmp+') has been expired while handling.' + geoloco, 'cancel', pid);
		showProblem('<strong>Event expired</strong>', 'There is nothing more to be done here.', 'Go home', 'homeNew');
	} else if(code == '410' && resultX && resultX.reason == 'cancelled') {
		archive('Event cancelled by the user', 'Requested ('+scopeTmp+') has been cancelled by the user.' + geoloco, 'cancel', pid);
		showProblem('<strong>Event cancelled by the user</strong>', 'There is nothing more to be done here.', 'Go home', 'homeNew');
	} else if(code == '410') {
		archive('Event handled by another application', 'Requested ('+scopeTmp+') has been expired while handling.' + geoloco, 'cancel', pid);
		showProblem('<strong>Event handled by another application</strong>', 'There is nothing more to be done here.', 'Go home', 'homeNew');
	} else if(code >= 500) {
		archive('Critical error', 'Service unavailable for a moment. Please try again.' + geoloco, 'cancel', pid);
		showProblem('<strong>Critical error</strong>', 'Service unavailable for a moment.', 'Go home', 'homeNew');
	} else {
		archive('Request failed', 'Requested ('+scopeTmp+') denied by API and access has not been granted.' + geoloco, 'cancel', pid);
		showProblem('Something went wrong', 'It looks like quite error happened. Please try again in a while.', 'Go home', 'homeNew');
	};
    _eventBeingHandled = false;
};
									

var pageContainer = document.getElementById('finalAPP');

function executeEvent(data, event) {
	
	consoleX('executeEvent started', 'Events');
	consoleX(JSON.stringify(data), 'Events');
	consoleX(JSON.stringify(event), 'Events');
	
	db.transaction(function(tx) {
		
		tx.executeSql('SELECT * FROM `Devices` WHERE `pid` = "'+data.pid+'"', [], function(tx, rs) {
			
			var len = rs.rows.length, i;
			
			if(len == 0) {
				//showProblem('Upsss<br><strong>PID not found</strong>', 'Error occured and operation has been terminated.', 'Go back', 'homeNew');
			};
			
			for (i = 0; i < len; i++) {
				
				var paired = rs.rows.item(i);
				
				if(data.result == 1) {
				
					consoleA(event, 'Event Payload');
					consoleA(paired, 'Pairing Payload');
					
					var eid_b64 = paired.eid;
					var aid_prv_b64 = paired.aid_prv;
					var aid_b64 = paired.aid;
					var jti_b64 = event.jti;
					var scope_enc_b64 = event.scope;

					//wycinamy MSG i MAC oraz pozbywamy sie ID algorytmu (czyli 'A' początkowego)
					var scope_enc_cut_bin = FromBase64( scope_enc_b64.substring(1) );
					var scope_enc_msg_b64 = ToBase64( scope_enc_cut_bin.slice(0, scope_enc_cut_bin.length - 32) );
					var scope_enc_mac_b64 = ToBase64( scope_enc_cut_bin.slice(scope_enc_cut_bin.length - 32) );
					consoleX('ENC_SCOPE ' + scope_enc_msg_b64);  //wiadomosc zaszyfrowana w base64
					consoleX('SCOPE_MAC ' + scope_enc_mac_b64);  //mac w base64 odszyfrowanej wiadomosci - oczekiwana wartosc

					//ECDH
					var remote_pub_bin = new Uint8Array( FromBase64( eid_b64 ) );
					var local_prv_bin = new Uint8Array( FromBase64( aid_prv_b64 ) );
					var secret_base64 = ToBase64( axlsign.sharedKey(local_prv_bin, remote_pub_bin) );
					consoleX( 'Wynik RAW ECDH ' +secret_base64 ); //tu jest wynik ECDH, stały dla danej pary kluczy

					//masterkey
					var hs1 = CryptoJS.HmacSHA256( CryptoJS.enc.Base64.parse(jti_b64), CryptoJS.enc.Base64.parse(secret_base64)) ; 
					var hmac_secret_b64 = hs1.toString(CryptoJS.enc.Base64); 
					consoleX( 'Klucz MASTER ' + hmac_secret_b64 );  //a tu mieszkamy (HMAC) wynik ECDH i JTI czyli mamy unikatowy klucz sesji

					//AES key & iv
					var master_key_bin = FromBase64( hmac_secret_b64 );
					var aes_key_b64 = ToBase64( master_key_bin.slice(0, 16) );
					var aes_iv_b64 = ToBase64( master_key_bin.slice(16) );
					consoleX('AES_KEY ' + aes_key_b64);    // klucz sesji pocieto na klucz AES (pierwsze 16bajtow)
					consoleX('AES_IV ' + aes_iv_b64);      //   oraz IV (ostatnie 16bajtow)

					var decipher_scope = CryptoJS.AES.decrypt( scope_enc_msg_b64, CryptoJS.enc.Base64.parse(aes_key_b64), {
					iv: CryptoJS.enc.Base64.parse(aes_iv_b64),
						mode: CryptoJS.mode.CBC
					});

					scope = decipher_scope.toString(CryptoJS.enc.Base64);  //deszyfrowany ciag jest w base64
					consoleX('Plaintext  ' +  scope);
					consoleX('Plaintext (string) ' +  atob(scope));  //a tu string ładny
					
					var scopeNow = atob(scope);
					var scopeTmp = scopeNow;
					var scopeRaw = false;
					var scopePayload = false;
					var issuer = event.ipinfo_eid;

					//mac wiadomosci
					var hs2 = CryptoJS.HmacSHA256( scopeNow, CryptoJS.enc.Base64.parse(hmac_secret_b64)) ; 
					var hmac_scope_b64 = hs2.toString(CryptoJS.enc.Base64);   //a tutaj mac deszyfrowanej wiadomosci i klucza sesji (calego, 32bajty)
					consoleX( 'MAC kontrolny  ' + hmac_scope_b64 );  
					consoleX( 'MAC oczekiwany ' + scope_enc_mac_b64 );  
					
					consoleA({ msg: scope, mac: hmac_scope_b64 }, 'After checking Payload');

					consoleX(JSON.stringify(event), 'FCM Handler after Validation');
					consoleX(JSON.stringify(paired), 'FCM Handler after Validation');	

					//generate JWT
					var jwt_secret_base64 = ToBase64( axlsign.sharedKey(local_prv_bin, remote_pub_bin) );
					consoleX("JWT Sec: " + jwt_secret_base64, 'genReply');
					
					// Scope hunting
					for(var key in _scopes) {
						if(key == scopeNow) {
							scopeRaw = scopeNow;
						};
					};
					
					if(!scopeRaw) {
						for(var key in _scopes) {
							if(!scopeRaw) {
								var re = new RegExp(key, "");
								var matching = scopeNow.match(re);
								if(matching != null) {
									scopeRaw = key;
									scopePayload = matching;
								};
							};
						};
					};

					var geoDataNow = ` Issuer geolocation: ${issuer.city}, ${issuer.country} with IP: ${issuer.ip}`;

					if(_scopes[scopeRaw]) {
						
						//archive('Request received', 'App received request from ' + rs.rows.item(i).user + ' for action with scope: ' + scopeRaw + ' at ' + Math.round(+new Date()/1000), 'ok');
						
						var scopeNow = JSON.parse(JSON.stringify(_scopes[scopeRaw]));
						
						if(_scopes[scopeRaw].question) {
							scopePayload = _scopes[scopeRaw].question(scopePayload);
						};
						
						for(var i = 1; i < scopePayload.length; i++) {
							scopeNow.question_header = scopeNow.question_header.replaceAll('%' + i, scopePayload[i]);
							scopeNow.question_description = scopeNow.question_description.replaceAll('%' + i, scopePayload[i]);
						};
						
						var rnd = 'opts_' + Math.ceil(Math.random()*56784365384);
						scopeNow.question_description += `${issuer.city}, ${issuer.country}<br>${issuer.ip}`;
						scopeNow.question_description += `<h4 class="toggle" rel="${rnd}"><i class="icon-cog"></i> Options <i class="icon-angle-down"></i></h4><div id="${rnd}" class="options toggler dead">
						<div class="form-radio form-radio-full form-radio-inline animatedX moved">
							<div class="form-radio-legend">Access grant period:</div>
							<label class="form-radio-label">
								<input name=accessGrantPeriod class="form-radio-field" type="radio" required value="" checked />
								<i class="form-radio-button"></i>
								<span>15m</span>
							</label>
							<label class="form-radio-label">
								<input name=accessGrantPeriod class="form-radio-field" type="radio" required value="1" />
								<i class="form-radio-button"></i>
								<span>1h</span>
							</label>
							<label class="form-radio-label">
								<input name=accessGrantPeriod class="form-radio-field" type="radio" required value="8" />
								<i class="form-radio-button"></i>
								<span>8h</span>
							</label>
							<label class="form-radio-label">
								<input name=accessGrantPeriod class="form-radio-field" type="radio" required value="24" />
								<i class="form-radio-button"></i>
								<span>24h</span>
							</label>
						</div></div>`;

						showConfirm(scopeNow.question_header, scopeNow.question_description, function(buttona, buttonb){
							
							//archive('Request accepted', 'User accepted request ('+scopeTmp+') and event has been send to Encedo API.', 'ok');
							
							var selectedWritable = document.querySelector('input[name="writable"]:checked');
							
							scopeTmp = scopeTmp.replace(':rw', '');
							
							if(selectedWritable) {
								scopeTmp = scopeTmp.concat(':rw');
							};
							
							consoleX('Scope now: ' + scopeTmp);
							
							var rep_mac_bin = CryptoJS.HmacSHA256(scopeTmp, CryptoJS.enc.Base64.parse(hmac_secret_b64));
							
							consoleX( 'stop 0');
							
							var rep_enc_scope_bin = CryptoJS.AES.encrypt( scopeTmp, CryptoJS.enc.Base64.parse(aes_key_b64), {
								iv: CryptoJS.enc.Base64.parse(aes_iv_b64),
								mode: CryptoJS.mode.CBC
							});
							
							consoleX( 'stop 1');   
							
							var rep_enc_scope_b64 = rep_enc_scope_bin;
							
							consoleX( 'stop 2'); 
							
							var rep_mac_b64 = rep_mac_bin.toString(CryptoJS.enc.Base64);
							
							consoleX( 'stop 3'); 
							consoleX( 'm1  ' + rep_enc_scope_b64 );  
							consoleX( 'm2 ' + rep_mac_b64 );  
							
							consoleX( 'stop 4'); 

							var m1 = atob(rep_enc_scope_b64);
							
							consoleX( 'stop 5'); 
							var m2 = atob(rep_mac_b64);
							
							consoleX( 'stop 6'); 
							var m  = m1.concat(m2);
							
							consoleX( 'stop 7'); 
							
							consoleX( 'm1  ' + rep_enc_scope_b64 );  
							consoleX( 'm2 ' + rep_mac_b64 );  
							
							var encoded_scope = 'A' + btoa(m);
							
							consoleX( 'A' + btoa(m) ); 
							
							consoleX( 'stop 8');
							
							var now = Math.round(+new Date()/1000);
							var expo = now + 15*60;
							var selectedTokenTime = document.querySelector('#' + rnd + ' input[name="accessGrantPeriod"]:checked');
							
							if(selectedTokenTime && selectedTokenTime.value && selectedTokenTime.value > 0) {
								expo = now + selectedTokenTime.value *60 * 60;
							};
							
							var dataX = {
								'aud': paired.eid,
								'jti': event.jti,
								'exp': expo,
								'iat': now,
								'pid': paired.pid,
								'iss': paired.aid,
								'scope': encoded_scope
							};

							var headerX = { 
							  "ecdh": "x25519"
							};

							var jwt = jwt_generate_hs256(headerX, dataX, CryptoJS.enc.Base64.parse(jwt_secret_base64));  
							consoleA({jwt: jwt}, 'genReply');	
							consoleX(jwt, 'genReply');	
							
							var remote_pub_bin2 = new Uint8Array( FromBase64( event.epk ) );
							var secret_base64 = ToBase64( axlsign.sharedKey(local_prv_bin, remote_pub_bin2) );
							
							var reply_mac = CryptoJS.HmacSHA256( jwt, CryptoJS.enc.Base64.parse( secret_base64) ) ;
							var reply_mac_b64 = reply_mac.toString(CryptoJS.enc.Base64);
							
							api('https://api.encedo.com/notify/event/data/' + data.eventId + '/' + data.type, function(status, resultX, code){
								
								archiveEvent(data.eventId);
								
								if(status) {
									archive(scopeNow.question_header, 'Requested ('+scopeTmp+') <strong>confirmed</strong> by user and access has been granted.' + geoDataNow, 'ok', data.pid);
									if(scopeNow.success) scopeNow.success();
									showOK(scopeNow.success_header, scopeNow.success_description, scopeNow.success_button, scopeNow.success_destination);
								} else {
									
									handleEventError(code, resultX, scopeTmp, data.pid, geoDataNow);
									
								};
								
								consoleA(resultX, 'AfterSending');
								
							}, 'POST', { authreply: jwt, mac: reply_mac_b64 } );
							
						}, function(){
							
							archiveEvent(data.eventId);
							archive(scopeNow.question_header, 'User <strong>denied</strong> request ('+scopeTmp+') and event has been terminated.' + geoDataNow, 'ok', data.pid);
							
							api('https://api.encedo.com/notify/event/data/' + data.eventId + '/' + data.type, function(status, resultX, code){
								
								if(status) {
									showAttention(scopeNow.deny_header, scopeNow.deny_description, scopeNow.deny_button, scopeNow.deny_destination);
								} else {
									handleEventError(code, resultX, scopeTmp, data.pid, geoDataNow);
								};
								
								consoleA(resultX, 'AfterSending');
								
							}, 'DELETE', {} );
							
						});	
					
					} else {
						
						archiveEvent(data.eventId);
						
						archive('Request received', 'App received request from ' + rs.rows.item(i).user + ' for action with unknown scope: ' + scopeRaw + ' at ' + dataX.iat + '. ' + geoDataNow, 'ok', data.pid);

						consoleX('Scope: ' + scopeRaw + ' has not been found. Please try again with another scope.', 'Scopes');
						
						changePage('homeNew');
						
					};

				} else if(data.result == 2) {
					removePairing(data.pidx);
					archiveEvent(data.eventId);
					consoleX('DELETE with ' + JSON.stringify(paired), 'Events');
				} else {
					archiveEvent(data.eventId);
					consoleX('Nothing to do: ' + JSON.stringify(paired), 'Events');
				};

			};
			
		}, function(tx, error) {
			_eventBeingHandled = false;
			showProblem('Upsss<br><strong>Invalid scope</strong>', 'Error occured and operation has been terminated.', 'Go back', 'homeNew');
		  consoleX('SELECT error: ' + error.message, 'SQLite');
		});
	});
};

function removePairing(pid) {
	
	db.transaction(function(tx, rs) {
		
		tx.executeSql('SELECT * FROM `Devices` WHERE `pid` = "'+pid+'"', [], function(tx, rs) {
		
			var found = [];
			var len = rs.rows.length, i;
			for (i = 0; i < len; i++) {
				var element = rs.rows.item(i);
				
				db.transaction(function(tx) {
					tx.executeSql('DELETE FROM `Devices` WHERE `pid` = "'+pid+'"', []);
				}, function(error) {
					consoleX('Device NOT unpaired', 'Pairing');
					consoleX('Transaction ERROR: ' + error.message, 'SQLite/Events');
				}, function() {
					printDevices();
					consoleX('Device unpaired', 'Pairing');
					archive('Device unpaired', 'This device has been successfully unpaired from the Encedo\'s manager.', 'ok', pid);
					showOK('<strong>DEVICE UNPAIRED</strong>', 'You need to manually remove this device in Encedo Manager.', 'ok', 'homeNew');
				});
				
			};
			
		}, function(tx, error) {
		  consoleX('SELECT error: ' + error.message, 'SQLite');
		});
		
	}, function(error) {
		consoleX('Checking devices to handle but there is no device.', 'Event Handling');
		consoleX('Transaction ERROR: ' + error.message, 'SQLite/Events');
	}, function() {
		consoleX('Checking devices to handle.', 'Event Handling');
	});
		
	
	
};

//const checkEventsToHandle3 = throttle(checkEventsToHandleFinal, 300);
var _eventBeingHandled = false;
var checkEventsToHandler = false;

function checkEventsToHandle(func) {
	//checkEventsToHandle(func);
	
	if(!_production) {
		if(func) func();
		return false;
	};
	
	if(_blocked) {
		return false;
	};
    
    if(_eventBeingHandled) {
        return false;
    };
	
	clearTimeout(checkEventsToHandler);
	
	checkEventsToHandler = setTimeout(function(){
		
		db.transaction(function(tx, rs) {
			tx.executeSql('SELECT * FROM `Devices`', [], function(tx, rs) {
			
				var found = [];
				var len = rs.rows.length, i;
				for (i = 0; i < len; i++) {
					var element = rs.rows.item(i);
					found.push(element.pid);
				};
				  
				api('https://api.encedo.com/notify/event/data/allbypid', function(status2, resultX2){
					
					if(resultX2 && resultX2.eventid && !Array.isArray(resultX2.eventid)) {
						
						consoleX('We found something to handle!', 'Event Handling');

						var already = false;
						
						for(const event in resultX2.eventid) {
                            
                            consoleX('Checking: ' + event, 'Event Handling');
							
							if(!already && !_eventBeingHandled) {
							
								let pids = String(resultX2.eventid[event]);
								
								let pidsx = pids.replaceAll('/', '_').replaceAll('+', '-').replaceAll('=', '');
								
								consoleX('Checking event ' + event + ' for ' + pidsx, 'Event Handling');
								
								already = true;
								_eventBeingHandled = event;
							
								api('https://api.encedo.com/notify/event/data/' + event + '/' + pidsx, function(status, eventon, code){
									if(status) {

                                        let expired = Math.floor(Date.now() / 1000) - eventon.exp;
                                        
                                        if(expired > 0) {
                                            
                                            consoleX(event, 'Event Garbage Collector');
                                            
                                            api('https://api.encedo.com/notify/event/data/' + event + '/' + pidsx, function(status, resultX, code){
                                                
                                                _eventBeingHandled = false;
                                                
                                                archiveEvent(event);
                                                archive(scopeNow.question_header, 'Event expired ' + geoDataNow, 'ok', pids);
                                                
                                                setTimeout(function(){
                                                    checkEventsToHandle(func);
                                                }, 10);
                                                
                                            }, 'DELETE', {} );
                                            
                                        } else {
                                            
                                            consoleX('Starting with event: ' + event, 'Event Handling');
                                            
                                            let eventPrototype = { eventId: event, pid: pids, name: 'Notification', type: pidsx, result: 1 };
                                            
                                            executeEvent(eventPrototype, eventon);
                                            
                                        };
                                        
									} else {
										handleEventError(code, eventon, 'Unknown yet');
									};
								});
							
                            } else {
                                
                                consoleX('Event rejected: ' + event + (already ? ' because another event is being handled ' : ''), 'Event Handling');
                                
                                if(_eventBeingHandled) {
                                    consoleX('Event handled right now is ' + _eventBeingHandled, 'Event Handling');
                                };

                            };
							
						};
						
					} else {
						
						consoleX('There are no events to handle!', 'Event Handling');
					
						if(func) func();
						
					};
					
				}, 'POST', { pid: found } );
				
				if(found.length == 0) {
					
					consoleX('There are no paired devices.', 'Event Handling');
					
					if(func) func();
					
				};
				
			}, function(tx, error) {
			  consoleX('SELECT error: ' + error.message, 'SQLite');
			  if(func) func();
			});
			
		}, function(error) {
			consoleX('Checking events to handle but there is no device.', 'Event Handling');
			consoleX('Transaction ERROR: ' + error.message, 'SQLite/Events');
			if(func) func();
		}, function() {
			consoleX('Checking events to handle.', 'Event Handling');
			if(func) func();
		});

	}, 1);
		
};

function checkEventsToHandleFinalOld(func) {
	
	if(_blocked) {
		consoleX('Application is blocked so no checking.', 'Events');
		return;
	} else {
		if(!_started) {
			if(func) func(false);
			return;
		};
		consoleX('Checking is there any event to handle!', 'Events');
	};
	
	db.transaction(function(tx) {
		tx.executeSql('SELECT * FROM Events WHERE `result` != 0', [], function(tx, rs) {
			
			var len = rs.rows.length;
			if(len > 0) {
				
				if(rs.rows.item(0).result == 1 && _eventBeingHandled != rs.rows.item(0).eventId) {
					
					consoleX('Event found to handle!', 'Events');
						
					api('https://api.encedo.com/notify/event/data/' + rs.rows.item(0).eventId + '/' + rs.rows.item(0).type, function(status, event, code){
						if(status) {
							_eventBeingHandled = rs.rows.item(0).eventId;
							executeEvent(rs.rows.item(0), event);
						} else {
							checkEventsToHandle(func);
							archiveEvent(rs.rows.item(0).eventId);
						};
					});
				
				} else {
					executeEvent(rs.rows.item(0), event);
				};
			
			} else {
				if(func) func(false);
			};
			
		}, function(tx, error) {
		  consoleX('SELECT error: ' + error.message, 'SQLite/Events');
		});
	});
};

function archiveEvent(id) {
	
	_eventBeingHandled = false;
	
	db.transaction(function(tx) {
		tx.executeSql('INSERT INTO `Events` (eventId, pid, name, type, result) VALUES (?, ?, ?, ?, ?)', [id, '', '', '', 1]);
	}, function(error) {
		consoleX('Transaction ERROR: ' + error.message, 'SQLite/Events');
	}, function() {
		consoleX('Event '+id+' has been marked as finished!', 'Events');
		checkEventsToHandle();
	});
	
};

function handleEvent(payload) {
	
	consoleX(payload.encedo, 'Events');
	
	var data = JSON.parse(payload.encedo);
	data.passer = 0;
	
	if(data.event) {
		data = data.event;
		data.pidA = Base64DecodeUrl(data.pid);
		data.pidx = data.pidA.replace('===', '==').replace('==', '=');
		data.passer = 1;
		
		checkEventsToHandle();
		
		setTimeout(function(){
			checkEventsToHandle();
		}, 1200);
	};
	
	if(data.pairing) {
		data = data.pairing;
		data.pidA = Base64DecodeUrl(data.pid);
		data.pidx = data.pidA.replace('===', '==').replace('==', '=');
		data.eventid = 'PAIRING' + Math.ceil(Math.random()*7432864789236);
		if(data.status == 'DELETED' && data.pid) {
			data.passer = 2;
			removePairing(data.pid);
		} else {
			data.passer = 3;
		};
	};
	
	return;

};

function validateResponse(data, func) {
	if(!data || !func) {
		func(false, data);
		return false;
	}

	func(true);
}

function parseQRCode(code) {
		
	code = JSON.parse(code);
	
	consoleX('Works 1', 'parseQRCode');
	
	api(code.link, function(status, data){
		
		consoleX('Works 2', 'parseQRCode');
		
		consoleX(JSON.stringify(data));
		
		consoleX('Works 3', 'parseQRCode');
		
		consoleX(data.request);
		
		consoleX('Works 4', 'parseQRCode');
		
		var parsed = parseJwt(data.request);
		
		consoleX('Works 1', 'JWT Parser');
		
		consoleX(JSON.stringify(parsed), 'JWT Parser');
		
		consoleX('Works 2', 'JWT Parser');
		
		var array = new Uint8Array(32);
		window.crypto.getRandomValues(array);
		var aid_key = axlsign.generateKeyPair( array );
		
		var fid = tokenNOW;
		var label = device.model + ' (' + device.platform + ')';
		var reply = genReply(parsed, aid_key, label, fid);
		var iat = Math.round(+new Date()/1000);
		var eid = reply.eid;
		var aid = reply.aid;
		var aid_prv = reply.aid_prv;
		
		
		db.transaction(function(tx, rs) {
		
			tx.executeSql('SELECT * FROM `Devices` WHERE `eid` = "'+eid+'"', [], function(tx, rs) {
			
				var found = [];
				var len = rs.rows.length, i;
				if(len > 0) {
					
					showAttention(
						'<strong>Already paired</strong>', 
						'You are already paired with this device. You cannot pair twice.', 
						'Go back', 
						function(){
							changePage('dashboard');
						}
					);
					
					api(code.link, function(status, data){ }, 'DELETE', {});
					
				} else {
					
					consoleX('Works 3', 'JWT Parser');
					
					var issuer = data.ipinfo_eid;
		
					showConfirm('Do you want to<br><strong>pair with<br>'+code.user+'?</strong>', 'Hostname: ' + code.hostname + (code.email && code.email.length > 1 ? '<br>Email: ' + code.email : '') + '<br><br>' + `${issuer.city}, ${issuer.country}<br>${issuer.ip}`, function(button_ok, button_cancel){
						
						if(button_ok) button_ok.innerHTML = 'Wait...';
						
						consoleX('Waiting for confirmation...', 'UI');
						
						api(code.link, function(status, data){
							
							if(status) {
							
								consoleX("Pairing complete!", 'Pairing');
								var when = formatWhen();
								
								if(db) {
									
									db.transaction(function(tx) {
										tx.executeSql('INSERT INTO Devices VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)', [data.pid, eid, aid_prv, aid, iat, when, code.user, code.email, code.hostname]);
										consoleX("Transaction started!", 'SQLite');
										consoleX('Inserted data: ' + data.pid + ' ' + when + ' ' + code.user + ' ' + code.email + ' ' + code.hostname, 'SQLite');
									  }, function(error) {
										consoleX('Transaction ERROR: ' + error.message, 'QRScanner');
										archive('Problem occured', 'Application tried to save new device owned by ' + code.user + (code.email && code.email.length > 0 ? ' (' + code.email + ')' : '') + ' but some kind of error has occured.', 'cancel', data.pid);
										
										showProblem(
											'<strong>Problem occured</strong>', 
											'Application tried to save new device owned by ' + code.user + (code.email && code.email.length > 0 ? ' (' + code.email + ')' : '') + ' but some kind of error has occured.', 
											'Go back', 
											function(){
												changePage('dashboard');
											}
										);
							
									  }, function() {
										consoleX('Populated database with code: ' + JSON.stringify(data), 'QRScanner');
										archive('Device successfully connected', 'New device owned by ' + code.user + (code.email && code.email.length > 0 ? ' (' + code.email + ')' : '') + ' has been successfully connected with this application.', 'ok', data.pid);
										//printDevice(data.pid, when, code.user, code.email, code.hostname);
										printDevices();
										showOK(
											'<strong>Device connected</strong>', 
											'New device owned by ' + code.user + (code.email && code.email.length > 0 ? ' (' + code.email + ')' : '') + ' has been successfully connected with this application.', 
											'OK', 
											function(){
												changePage('dashboard');
											}
										);
										
									});	
									
								} else {
									
									showOK(
										'<strong>Device connected</strong>', 
										'New device has been successfully connected with this application.', 
										'OK', 
										function(){
											changePage('dashboard');
										}
									);
								
								};
							
							} else {
								showProblem(
									'<strong>Problem occured</strong>', 
									'Application tried to save new device but some kind of error has occured.', 
									'Go back', 
									function(){
										changePage('dashboard');
									}
								);
							}
							
						}, 'POST', reply.payload);			
						
					}, function(){

						changePage('dashboard');
						archive('Device not connected', 'This application was ready to connect with Encedo Dashboard but user declined.', 'cancel', data.pid);
						api(code.link, function(status, data){}, 'DELETE', reply);	
						
					});
		
					
					
				};
				
			}, function(tx, error) {
			  consoleX('SELECT error: ' + error.message, 'SQLite');
			});
			
		}, function(error) {
			consoleX('Checking devices to handle but there is no device.', 'Event Handling');
			consoleX('Transaction ERROR: ' + error.message, 'SQLite/Events');
		}, function() {
			consoleX('Checking devices to handle.', 'Event Handling');
		});
		
		consoleX(reply);
	});
	
};

function archive(header, text, status, pid) {
	var when = formatWhen();
	if(!pid) pid = '';
	
	header = header.replaceAll('Grant access to', '').replaceAll('<strong>', '').replaceAll('</strong>', '').replaceAll('<br>', '');
	
	if(db) {
		db.transaction(function(tx) {
			tx.executeSql('INSERT INTO Archives (name, desc, status, date, pid) VALUES (?, ?, ?, ?, ?)', [header, text, status, when, pid]);
		}, function(error) {
			consoleX('Transaction ERROR: ' + error.message, 'SQLite');
		}, function() {
			archivePrint(header, text, status, when, pid);
			consoleX('New archive element created.', 'SQLite');
		});
	} else {
		archivePrint(header, text, status, when, pid);
	};
};

var dateToday = new Date();
var dateYesterday = new Date();
dateYesterday.setDate(dateYesterday.getDate() - 1);

var alreadyHeaderToday = false;
var alreadyHeaderYesterday = false;

function archivePrint(header, text, status, when, pid) {
	if(!archiveStarted) {
		archiveStarted = true;
		alreadyHeaderToday = false;
		alreadyHeaderYesterday = false;
		archiveElements.innerHTML = '';
	};
	
	let dayHere = when.split(' ')[1];
	dayHere = parseInt(dayHere.split('/')[0]);
	
	archiveElements.insertAdjacentHTML('afterbegin', '<div class="item animatedX moved"><div class="data">'+when.split(' ')[0]+'<br>'+when.split(' ')[1]+'<br><span>'+when.split(' ')[2]+'</span><br><i class="icon-'+status+' animatedX"></i></div><div class="desc"><p><strong>'+header+'</strong><span>'+text+'</span></p></div></div>');
	
	if(dateToday.getDate() == dayHere && !alreadyHeaderToday) {
		//archiveElements.insertAdjacentHTML('afterbegin', '<div class="animatedX moved"><p>TODAY</p></div>');
		alreadyHeaderToday = true;
	};
	
	if(dateYesterday.getDate() == dayHere && !alreadyHeaderYesterday) {
		//archiveElements.insertAdjacentHTML('afterbegin', '<div class="item animatedX moved"><p>YESTERDAY</p></div>');
		alreadyHeaderYesterday = true;
	};
};

function printDevice(pid, when, user, email, host) {
	consoleX('Device connected to '+user+': ' + pid + ' at ' + when, 'Devices');
	devicesElements.insertAdjacentHTML('afterbegin', '<div class="item animatedX moved" rel="devicePageOpen/'+pid.replaceAll('/', '-')+'"><i class="icon-ellipsis makeAction" style="float: right; font-size: 220%; margin-top: 10px; margin-right: -5px; color: #6E358C;" rel="devicePageOpen/'+pid.replaceAll('/', '-')+'"></i><div class="desc"><p><strong>'+user+'</strong><span class="liner"></span><span>Hostname: '+host+' '+(email && email.length ? '<br>Email: ' + email : '')+'</span><br>Added: '+when.split(' ')[1]+'/'+when.split(' ')[0]+' '+when.split(' ')[2]+'</p></div></div>');
	devicesTabs.insertAdjacentHTML('beforeend', '<li class="animatedX moved makeAction" rel="openArchiveForDevice/'+pid.replaceAll('/', '-')+'" data-pidx="'+pid.replaceAll('/', '-')+'"><span>'+user+'</span></li>');
};		
function printDevices() {
	
	db.transaction(function(tx) {
		tx.executeSql('SELECT * FROM `Devices`', [], function(tx, rs) {
			
			if(!devicesStarted) {
				devicesStarted = true;
			};
			
			var found = false;
			var len = rs.rows.length, i;
			for (i = 0; i < len; i++) {
				if(!found) {
					devicesElements.innerHTML = '';

					devicesTabs.innerHTML = '<li class="animatedX moved makeAction tab--active" rel="openArchiveForDevice" id="showArchiveAllEvents"><span>All events</span></li>';
					//devicesTabs.innerHTML += '<li class="animatedX moved makeAction" rel="showArchiveSearch" id="showArchiveSearch"><span><i class="icon-search"></i> Search</span></li>';

					document.getElementById('ifDevicesAlreadyDOM').classList.add('glued-bottom');
				};
				printDevice(rs.rows.item(i).pid, rs.rows.item(i).datetime, rs.rows.item(i).user, rs.rows.item(i).email, rs.rows.item(i).host);
				found = true;
			};
			
			if(!found) {
				devicesElements.innerHTML = '<p class="mainP animatedX moved"><strong>You do not have any connected devices right now.</strong></p><br><br>';
				document.getElementById('ifDevicesAlreadyDOM').classList.remove('glued-bottom');
			};
			
		}, function(tx, error) {
		  consoleX('SELECT error: ' + error.message, 'SQLite');
		});
	});		
};

var howManyChecks = 0;
var errorCallbackHandler = false;

function checkBlockage() {
	
	if(howManyChecks < 7) {
	
		Fingerprint.show({
		  title: 'Fingerprint Authentication',
		  description: "Application is locked. Please Sign on before you go any further."
		}, successCallback, errorCallback);

		function successCallback(){
		  consoleX("Authentication successful", 'FingerprintAuth');
		  consoleX("Last page: " + lastPage2, 'FingerprintAuth');
		  
		  _blocked = false;
		  _backgroundAllowed = false;
		  
		  clearTimeout(errorCallbackHandler);
		  
		  unblockApp();
		  checkEventsToHandle();
		  
		  setTimeout(function(){
		      checkEventsToHandle();
		  }, 1500);
		  
		  howManyChecks = 0;
		  
		  if(lastPage && lastPage != 'tplOk' && lastPage != 'tplConfirm' && lastPage != 'tplProblem' && lastPage != 'tplAttention') {
			  startFinalApp(lastPage);
		  } else {
			  startFinalApp('homeNew');
		  };
		};

		function errorCallback(error){
			
			clearTimeout(errorCallbackHandler);
			
			errorCallbackHandler = setTimeout(function(){
				
			  consoleX("Authentication invalid " + error.message, 'FingerprintAuth');
			  archive('Authentication problem', 'While trying to authenticate user we encountered an error with codename: ' + error.message, 'cancel');
			  howManyChecks++;
			  blockApp();
			  
			  setTimeout(function(){
				  //checkBlockage();
			  }, 2200);
			  
			  _blocked = true;
			  _backgroundAllowed = false;

			}, 5300);
			
		};
	
	} else {
		consoleX('Panic situation', 'FingerprintAuth');
		archive('Panic situation', 'Application crashed and all the could do is to just show error information.', 'cancel');
		//_blocked = false;
		//_backgroundAllowed = false;
		//unblockApp();
		errorApp();
	};
};

function prepareHandlers() {
	
	var console2 = document.getElementById('console');
    var body = document.getElementById('body');
    var shapes = document.getElementById('shapes');
    var finalAPP = document.getElementById('finalAPP');
    var consoleApp = document.getElementById('consoleApp');
    var consoleApp2 = document.getElementById('consoleApp2');
    var scannedQRs = document.getElementById('scannedQRs');
    var deviceready = document.getElementById('deviceready');
    var eraseAll = document.getElementById('eraseAll');
    var archiveElements = document.getElementById('archiveElements');
    var devicesTabs = document.getElementById('devicesTabs');
    var devicesElements = document.getElementById('devicesElements');
    var devicePageDetails = document.getElementById('devicePageDetails');
	var qrscannerFrame = document.getElementById('qrscannerFrame');
    var qrscannerFrameBack = document.getElementById('qrscannerFrameBack');
    var archiveSearchInput = document.getElementById('archiveSearchInput');

	bodyDOM = document.body;

	var goToTop = document.getElementById('goToTop');
	goToTop.onclick = function(){
		scrollTo(0,0);
	};
	
	if(_production) {
		
		var el = document.getElementById("getQRcode");

		el.addEventListener("click", function(){
			
			consoleX('Trying to scan QR Code.', 'QRScanner');
			
			_backgroundAllowed = true;
			
			QRScanner.prepare(onDone); // show the prompt
	 
			function onDone(err, status){
				
			  if (err) {
			   // here we can handle errors and clean up any loose ends.
			   consoleX(err, 'QRScanner');
			   consoleX(JSON.stringify(err), 'QRScanner');
			   showProblem('QR Scanner<br><strong>Problem</strong>', 'Some kind of a problem with QR Scanner', 'Try again', function(){
					changePage('dashboard');
				});
				
				_backgroundAllowed = false;

			  };
			  
			  if (status.authorized) {
				
				shapes.classList.add('hidden');
				body.classList.add('hidden');
				finalAPP.classList.add('hidden');
				qrscannerFrame.classList.add('active');
				
				QRScanner.scan(displayContents);

				// Make the webview transparent so the video preview is visible behind it.
                QRScanner.show(function(status){
                  consoleX(JSON.stringify(status), 'QRScannerShow');
                });
		 
				function displayContents(err, text){
					
				  if(err){

					showProblem('QR Scanner<br><strong>Problem</strong>', 'Some kind of a problem with QR Scanner', 'Try again', function(){
						changePage('dashboard');
					});
					consoleX(err, 'QRScanner');
					consoleX(text, 'QRScanner');
					consoleX(JSON.stringify(err), 'QRScanner');
					
					QRScanner.destroy(function(status){  console.log(status); body.classList.remove('hidden'); });
					
				  } else {
					  
					consoleX('Scanned with success!', 'QRScanner');  
					consoleX(text, 'QRScanner');  

					shapes.classList.remove('hidden');
					body.classList.remove('hidden');
					finalAPP.classList.remove('hidden');
					qrscannerFrame.classList.remove('active');
					parseQRCode(text);
					
					QRScanner.destroy(function(status){  console.log(status); body.classList.remove('hidden'); });

				  };
				  
				  _backgroundAllowed = false;
				  
				};
				
			  } else if (status.denied) {
			   // The video preview will remain black, and scanning is disabled. We can
			   // try to ask the user to change their mind, but we'll have to send them
			   // to their device settings with `QRScanner.openSettings()`.
				showProblem('QR Scanner<br><strong>Problem</strong>', 'This action need permission to access your device\'s camera.', 'Try again', function(){
					changePage('dashboard');
				});
				QRScanner.destroy(function(status){  console.log(status); body.classList.remove('hidden'); });
				_backgroundAllowed = false;
			  } else {
				// we didn't get permission, but we didn't get permanently denied. (On
				// Android, a denial isn't permanent unless the user checks the "Don't
				// ask again" box.) We can ask again at the next relevant opportunity.
				showProblem('QR Scanner<br><strong>Problem</strong>', 'This action need permission to access your device\'s camera.', 'Try again', function(){
					changePage('dashboard');
				});
				QRScanner.destroy(function(status){  console.log(status); body.classList.remove('hidden'); });
				_backgroundAllowed = false;
			  };
			};
			
		}, false);

	
	} else {
		
		var el = document.getElementById("getQRcode");
		el.addEventListener("click", function(){
			shapes.classList.add('hidden');
			body.classList.add('hidden');
			finalAPP.classList.add('hidden');
			qrscannerFrame.classList.add('active');
			_backgroundAllowed = true;
		});
		printDevice('tdbn382ydb238', '2021-12-12 12:00:00 ', 'Testing Device', false, 'my.ence.do');
		
	};
	
	qrscannerFrameBack.addEventListener("click", function(){
		console.log('test');
		shapes.classList.remove('hidden');
		body.classList.remove('hidden');
		finalAPP.classList.remove('hidden');
		qrscannerFrame.classList.remove('active');
		if(_production) QRScanner.destroy(function(status){  console.log(status); body.classList.remove('hidden'); });
		_backgroundAllowed = false;
	});

	var openArchiveForDeviceHandler = false;

	let archiveOpenedPID = false;
	let archiveSearchInputQuery = '';

	function refreshArchiveData() {
	    archiveSearchInputQuery = archiveSearchInput.value.trim();
	    if(archiveSearchInputQuery.length > 0) {
	        cleanArchiveSearchQuery.style.display = 'block';
	    } else {
	        cleanArchiveSearchQuery.style.display = 'none';
	    };
	    getArchives(archiveOpenedPID, archiveSearchInputQuery);
	};

	archiveSearchInput.addEventListener('keyup', refreshArchiveData);
	archiveSearchInput.addEventListener('keydown', refreshArchiveData);
	archiveSearchInput.addEventListener('change', refreshArchiveData);
	archiveSearchInput.addEventListener('paste', refreshArchiveData);

	function getArchives(pid, query) {

	    archiveElements.innerHTML = '<p class="mainP animatedX moved"><br><strong>Getting data...</strong><br><span class="loader-container"><span></span><span></span><span></span><span></span><span></span></span> &nbsp; <span id="encedo_init_status"></span> <br></p>';
        archiveStarted = false;

	    let pidQuery = '';

	    if(pid && pid.length < 1) {
            pid = false;
        };

	    if(query && query.length < 1) {
	        query = false;
	    };

	    if(pid || query) {
	        pidQuery = 'WHERE ';
	        if(pid) {
	            pidQuery += '`pid` = \''+pid+'\' ';
	        };
	        if(pid && query) {
	            pidQuery += ' AND ';
	        };
	        if(query) {
                pidQuery += ' (`name` LIKE \'%'+query+'%\' OR `desc` LIKE \'%'+query+'%\' )';
            };
	    };

		clearTimeout(openArchiveForDeviceHandler);

		openArchiveForDeviceHandler = setTimeout(function(){

            db.transaction(function(tx) {
                tx.executeSql('SELECT * FROM `Archives` ' + pidQuery, [], function(tx, rs) {
                    consoleX('SELECT * FROM `Archives` ' + pidQuery, 'SQLite');
                    var len = rs.rows.length, i;
                    var howMany = 0;
                    archiveElements.innerHTML = '';
                    for (i = 0; i < len; i++) {
                        archivePrint(rs.rows.item(i).name, rs.rows.item(i).desc, rs.rows.item(i).status, rs.rows.item(i).date);
                        howMany++
                    };
                    if(howMany == 0) {
                        archiveElements.innerHTML = '<p class="mainP animatedX moved"><strong>There is no data to show? Impossible! Perhaps the archives are incomplete.</strong></p>';
                    };
                }, function(tx, error) {
                  consoleX('SELECT error: ' + error.message, 'SQLite');
                });
            });

        }, 250);
	};

	register('cleanArchiveSearchQuery', function(pidRaw){
	    archiveSearchInputQuery = '';
    	archiveSearchInput.value = '';
    	refreshArchiveData();
	});

	register('openArchiveForDevice', function(pidRaw){

        let pid = false;
		let pidQuery = '';
		let timer = 50;
		let containerTab = document.getElementById('devicesTabs');
		let tabs = containerTab.getElementsByTagName('li');

		archiveSearchInputQuery = '';
		archiveSearchInput.value = '';
		
		for (i = 0; i < tabs.length; i++) {
			var obj = tabs[i];
			obj.classList.remove('tab--active');
		};
		
		if(pidRaw) {
			pid = pidRaw.replaceAll('-', '/');
			pidQuery = 'WHERE `pid` = \''+pid+'\' ';
			for (i = 0; i < tabs.length; i++) {
				var obj = tabs[i];
				if(obj && obj.dataset && obj.dataset.pidx == pidRaw) {
					obj.classList.add('tab--active');
				};
			};
			archiveOpenedPID = pid;
		} else {
		    timer = 350;
			tabs[0].classList.add('tab--active');
			archiveOpenedPID = false;
		};
		
		if(lastPage != 'archive') {
		    timer = 350;
			changePage('archive'); 
		};

		getArchives(archiveOpenedPID, archiveSearchInputQuery);
		
	});
	
	var deviceOpenedRightNow = false;
		
	register('devicePageOpen', function(pidRaw){
		
		pid = pidRaw.replaceAll('-', '/');
		
		consoleX('Looking for found with PID: ' + pid, 'devicePageOpen');
		
		db.transaction(function(tx, rs) {
			
			tx.executeSql('SELECT * FROM `Devices` WHERE `pid` = "'+pid+'"', [], function(tx, rs) {
			
				var len = rs.rows.length, i;
				for (i = 0; i < len; i++) {
					consoleX('Device found with PID: ' + pid, 'devicePageOpen');
					var element = rs.rows.item(i);
					deviceOpenedRightNow = element;
					element.pidx = element.pid.replaceAll('/', '-');

					element.kid = element.aid;
					element.kido = element.aid.substr(0, 15) + '... (unhide)';
					element.pido = element.pid.substr(0, 15) + '... (unhide)';

					if(!element.email || element.email.length < 2) {
						element.email = '-';
					};
					
					devicePageDetails.innerHTML = `<br><h3 class="animatedX moved makeAction" rel="changeDeviceName/${element.pidx}"><i class="icon-pencil" style="float: right; font-size: 90%; margin-top: 15px; margin-right: -5px; color: #6E358C;"></i><span>Device name</span><strong>${element.user}</strong></h3> 
					<h4 class="animatedX moved"><span>Hostname</span><strong>${element.host}</strong></h4> 
					<h4 class="animatedX moved makeAction" rel="changeDeviceEmail/${element.pidx}"><i class="icon-pencil" style="float: right; font-size: 90%; margin-top: 15px; margin-right: -5px; color: #6E358C;"></i><span>Email</span><strong>${element.email}</strong></h4>
					<h4 class="animatedX moved"><span>KID</span><strong class="changeTo" rel="${element.kid}">${element.kido}</strong></h4>
					<h4 class="animatedX moved"><span>PID</span><strong class="changeTo" rel="${element.pid}">${element.pido}</strong></h4>
					<br><a class="buttonCTA makeAction animatedX moved" rel="openArchiveForDevice/${element.pidx}">Device archive <i class="icon-book"></i></a><br><a class="buttonCTA buttonCTAG makeAction animatedX moved" rel="unpairDevice/${element.pidx}"><i class="icon-cancel"></i> Unpair device</a>
					`;
				};
				
			}, function(tx, error) {
			  consoleX('SELECT error: ' + error.message, 'SQLite');
			});
			
		}, function(error) {
			consoleX('Device NOT found: ' + pid, 'devicePageOpen');
			consoleX('Transaction ERROR: ' + error.message, 'SQLite/Events');
		}, function() {
			consoleX('Device details showed.', 'devicePageOpen');
			//archive('Device details opened', 'Details of the device has been opened.', 'ok');
			changePage('devicePage');
		});
	});
	
	register('changeDeviceName', function(pidRaw){
		
		pid = pidRaw.replaceAll('-', '/');
		
		var newName = prompt('What name do you want?', deviceOpenedRightNow.user);
		consoleX('Trying to change name for a device to ' + newName, 'SQLite');
		if(newName && newName.length > 2) {
			db.transaction(function(tx, rs) {
				
				tx.executeSql('UPDATE `Devices` SET `user` = \''+newName+'\' WHERE `pid` = "'+pid+'"', [], function(tx, rs) {
					
				}, function(tx, error) {
				  consoleX('SELECT error: ' + error.message, 'SQLite');
				});
			}, function(error) {
			consoleX('Device NOT found: ' + pid, 'devicePageOpen');
			consoleX('Transaction ERROR: ' + error.message, 'SQLite/Events');
		}, function() {
			printDevices();
			archive('Device details changed', 'Name of the device ('+deviceOpenedRightNow.user+') has been changed.', 'ok', pid);
			actions['devicePageOpen'](pid);
		});
		};
	});
	
	register('changeDeviceEmail', function(pidRaw){
		
		pid = pidRaw.replaceAll('-', '/');
		
		var newName = prompt('What email do you want?', deviceOpenedRightNow.email);
		consoleX('Trying to change email for a device to ' + newName, 'SQLite');
		if(newName && newName.length > 2) {
			db.transaction(function(tx, rs) {
				
				tx.executeSql('UPDATE `Devices` SET `email` = \''+newName+'\' WHERE `pid` = "'+pid+'"', [], function(tx, rs) {
					
				}, function(tx, error) {
				  consoleX('SELECT error: ' + error.message, 'SQLite');
				});
			}, function(error) {
			consoleX('Device NOT found: ' + pid, 'devicePageOpen');
			consoleX('Transaction ERROR: ' + error.message, 'SQLite/Events');
		}, function() {
			printDevices();
			archive('Device details changed', 'Email of the device ('+deviceOpenedRightNow.user+') has been changed.', 'ok', pid);
			actions['devicePageOpen'](pid);
		});
		};
	});
	
	register('unpairDevice', function(pidRaw){
		
		pid = pidRaw.replaceAll('-', '/');
		
		consoleX('Unpairing from APP started for <strong> ' + pidRaw + '</strong>', 'Pairing');
		
		showConfirm('<strong>Are you sure?</strong>', 'There is no going back ...', function(buttona, buttonb){

			consoleX('Unpairing from APP confirmed for <strong> ' + pidRaw + '</strong>', 'Pairing');
			
			db.transaction(function(tx, rs) {
				tx.executeSql('SELECT * FROM `Devices` WHERE `pid` = "'+pid+'"', [], function(tx, rs) {
				
					var found = false;
					var len = rs.rows.length, i;
					for (i = 0; i < len; i++) {
						consoleX('Device found to unpair.', 'Devices');
						var element = rs.rows.item(i);
						
						api('https://api.encedo.com/notify/session', function(status, resultX){
													
							var epk_b64 = resultX.epk;
							var eid_b64 = element.eid;
							var aid_prv_b64 = element.aid_prv;
							var aid_b64 = element.aid;

							var remote_pub_bin = new Uint8Array( FromBase64( epk_b64 ) );   //ziana z eid na epk
							var local_prv_bin = new Uint8Array( FromBase64( aid_prv_b64 ) );   //to juz masz z bazy, jest ok
							var secret_base64 = ToBase64( axlsign.sharedKey(local_prv_bin, remote_pub_bin) );   //to jest ok
							
							var nonce_bin = new Uint8Array(32);
							window.crypto.getRandomValues(nonce_bin);
							var nonce_base64 =  ToBase64( nonce_bin);
							
							var mac_bin  = CryptoJS.HmacSHA256(CryptoJS.enc.Base64.parse(nonce_base64), CryptoJS.enc.Base64.parse(secret_base64) );
							var mac_base64 = mac_bin.toString(CryptoJS.enc.Base64);
							
							// aid z aplikacja app id
							// eid z managera encedo id
							// epk to klucz sesji
							// pid to pairing id
							
												
							if(status) {
								
								api('https://api.encedo.com/notify/subscribers/delete', function(status2, resultX2){
									
								}, 'POST', { pid: element.pid, epk: resultX.epk, nonce: nonce_base64, mac: mac_base64, aid: aid_b64 } );
								
								removePairing(pid);
									
							} else {
								showProblem('<strong>Device has NOT been unpaired</strong>', 'Some error occured. Device is still paired.', 'Ok', 'dashboard');
							};
							
						}, 'POST', { aid: element.aid } );

						found = true;
					};
					
					if(!found) {
						showProblem('<strong>Device has NOT been found</strong>', 'Some error occured. Device has not been found sadly.', 'Ok', 'dashboard');
					};
					
				}, function(tx, error) {
				  consoleX('SELECT error: ' + error.message, 'SQLite');
				  showProblem('<strong>Device has NOT been unpaired</strong>', 'Some error occured. Device is still paired.', 'Ok', 'dashboard');
				});
				
			}, function(error) {
				consoleX('Device unpairing fail. Device NOT found: ' + pidRaw, 'Pairing');
				consoleX('Transaction ERROR: ' + error.message, 'SQLite/Events');
				showProblem('<strong>Device has NOT been unpaired</strong>', 'Some error occured. Device is still paired.', 'Ok', 'dashboard');
			}, function() {
				consoleX('Device unpairing complete.', 'Pairing');
			});

			
		}, function(){
			
			consoleX('Unpairing from APP denied for <strong> ' + pidRaw + '</strong>', 'Pairing');
			//changePage('dashboard');
			changePage(lastPage2);
			
		});

	});

	var consoleLogButton = document.getElementById('consoleLogButton');
	
	register('startCounter', function(){
		superCounter++;
		if(superCounter > 10) {
			consoleLogButton.style.display = 'grid';
		};
	});
	
	register('unlockApp', function(){
		
		if(_isFinger) {
			checkBlockage();
		} else {
			
			_blocked = false;
		    _backgroundAllowed = false;
			
			consoleX((_blocked ? 'Blocked' : 'Not blocked'), 'unlockApp');
			if(lastPage && lastPage != 'tplOk' && lastPage != 'tplConfirm' && lastPage != 'tplProblem' && lastPage != 'tplAttention') {
				startFinalApp(lastPage);
			} else {
				startFinalApp('homeNew');
			};
			
			checkEventsToHandle();
			
			setTimeout(function(){
				checkEventsToHandle();
			}, 1500);
			
		};
		
	});
	
};
