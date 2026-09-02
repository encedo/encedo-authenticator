// shortcut for invoking activities' actions
function makeAction(name, arg) {
	if(actions[name]) actions[name](arg);
};

function register(name, func) {
	actions[name] = func;
};

const onScroll = function(where) {
	if(where > 100) {
		if(!already) {
			if(bodyDOM) bodyDOM.classList.add('changed');
			already = true;
		};
	} else {
		if(bodyDOM) bodyDOM.classList.remove('changed');
		already = false;
	};
};

const onScrollHandler = throttle(onScroll, 200);	
let ticking = false;
let last_known_scroll_position = 0;

window.addEventListener('scroll', (e) => {
  last_known_scroll_position = window.scrollY;
  if (!ticking) {
	window.requestAnimationFrame( () => {
		onScrollHandler(last_known_scroll_position);
	  ticking = false;
	});
	ticking = true;
  }
}, { capture: false, passive: true });

document.addEventListener("backbutton", onBackKeyDown, false);

var pageStack = [];

function getLastPage() {
	return pageStack.pop();
};

function onBackKeyDown() {

    consoleX('Back button has been pressed!', 'CordovaUI');

    if(_backgroundAllowed) {
        shapes.classList.remove('hidden');
        body.classList.remove('hidden');
        finalAPP.classList.remove('hidden');
        qrscannerFrame.classList.remove('active');
        if(_production) QRScanner.destroy(function(status){  console.log(status); body.classList.remove('hidden'); });
        _backgroundAllowed = false;
    } else {
        if(lastPage2 && lastPage2 != 'blocked' && lastPage2 != 'ok' && lastPage2 != 'confirm' && lastPage2 != 'problem' && lastPage2 != 'attention') {
    		var lastPP = getLastPage();
            consoleX(lastPP, 'CordovaUI');
            changePage(lastPP, true);
    		//history.back();
    	};
    };

};

window.onpopstate = function(event) {
  consoleX(`state: ${JSON.stringify(event.state)}`);
  changePage(event.state.page);
}

window.addEventListener('blur', function(){
	consoleX("Application blurred.", 'UI');
});

window.addEventListener('focus', function(){
	consoleX("Application focused.", 'UI');
	if(!_blocked) {
		checkEventsToHandle();
	};
});

document.addEventListener('pause', function(){
	if(_blockable) {
		_blocked = true;
		blockApp();
	}
	consoleX("Application paused.", 'UI');
});

document.addEventListener('resume', function(){
	consoleX("Application resumed.", 'UI');
	consoleX((_blocked ? 'Blocked' : 'Not blocked'), 'UI/Resume');
	if(!_blocked) {
		setTimeout(function(){
			checkEventsToHandle();
		}, 10);
	};
});

function closeAllPages(exclude) {
	var pages = pageContainer.querySelectorAll(".page:not(#"+exclude+")");
	pages.forEach(function(item){
		item.classList.remove('pageVisible');
		item.classList.remove('pageActive');
	});
};

var changePageHandler = false;
var changePageHandlerA = false;
var changePageHandlerB = false;
var changePageHandlerC = false;

function changePage2(name, func, force) {

	if(!force) force = false;

	if(name == 'blocked' && !_blockable) {
		return false;
	};

	var elementNow = document.getElementById(name);

	clearTimeout(changePageHandler);
	clearTimeout(changePageHandlerA);
	clearTimeout(changePageHandlerB);
	clearTimeout(changePageHandlerC);

	changePageHandler = setTimeout(function(){

		elementNow.classList.add('pageVisible');

		if(lastClass) {

			if(elementNow && elementNow.dataset && elementNow.dataset.class) {
				if(lastClass != elementNow.dataset.class) {
					pageContainer.classList.remove(lastClass);
					body.classList.remove(lastClass);
				};
			} else {
				pageContainer.classList.remove(lastClass);
				body.classList.remove(lastClass);
			};
		};

		if(lastPage) {
			document.getElementById(lastPage).classList.remove('pageActive');
		};

		changePageHandlerA = setTimeout(function(){
			elementNow.classList.add('pageActive');
			if(func){
				func();
			};
		}, 50);

		changePageHandlerB = setTimeout(function(){
			//closeAllPages(name);
			window.scrollTo(0, 0);
		}, 500);

		changePageHandlerC = setTimeout(function(){
			if(lastPage) {
				document.getElementById(lastPage).classList.remove('pageVisible');
			};

            if ("pushState" in history) {
			    history.pushState( { page: lastPage2 } , "", "");
			};

			lastPage2 = lastPage;
			lastPage = name;

			if(elementNow.dataset && elementNow.dataset.class) {
				pageContainer.classList.add(elementNow.dataset.class);
				body.classList.add(elementNow.dataset.class);
				lastClass = elementNow.dataset.class;
				consoleX("Page changed to <strong>"+name+"</strong> and main frame class changed to  " + lastClass, 'UI');
			};
			
		}, 850);

		
	}, 10);

	
};

var pageChangerHandler = false;

function changePage(name, preventFromHistory) {
		
    var clsNew = '';
    var titleNow = '';
    var elementNow = document.getElementById(name);
    var elementBefore = false;
    
    if(!elementNow) return;
    
    if(elementNow && elementNow.dataset && elementNow.dataset.class) {
        var clsNew = elementNow.dataset.class.split(' ');
    };
    
    elementNow.classList.add('pageVisible');
    
    if(lastPage) {
        makeAction('__' + lastPage);
        elementBefore = document.getElementById(lastPage).classList;
        elementBefore.remove('pageActive');
    };
    
    if(lastClass) {
        var cls = lastClass.split(' ');
        cls.forEach(function(e){
            if(!clsNew.includes(e)) {
                pageContainer.classList.remove(e);
                body.classList.remove(e);
            };
        });
    };
    
    setTimeout(function(elementNow){
        if(elementBefore) {
            elementBefore.remove('pageVisible');
            elementNow.classList.add('pageVisible');
        };
    }, 400, elementNow);
    
    clearTimeout(pageChangerHandler);

    pageChangerHandler = setTimeout(function(elementNow){
    
        window.scrollTo(0, 0);
        closeAllPages(name);
        elementNow.classList.add('pageActive');

        if ("pushState" in history) {
            history.pushState( { page: lastPage } , "", "");
        };

        if(preventFromHistory) {
            consoleX('No history push!', 'CordovaUI');
        } else {
            pageStack.push(lastPage);
        };
        
        makeAction(name);
        lastPage2 = lastPage;
        lastPage = name;
        
        if(elementNow.dataset){
        
            if(elementNow.dataset.class) {
                clsNew.forEach(function(e){
                    body.classList.add(e);
                    pageContainer.classList.add(e);
                });
                lastClass = elementNow.dataset.class;
            };
            
        };
        
    }, 600, elementNow);
    
};

function makeProgressB(element, start, end, label, time) {
	var x = this;
	var container = document.createElement("div");
	var bar = document.createElement("div");
	var valuer = document.createElement("span");
	bar.classList.add('bar');
	container.classList.add('progressB');
	element.innerHTML = '';
	element.appendChild(container);
	container.appendChild(bar);
	bar.appendChild(valuer);
	
	var clicker = 0;
	var steps = end-start;
	var ticker = steps/time;

	var summator = start;
	
	x.set = function(value) {
		bar.style.width = value + '%';
		valuer.innerHTML = value.toFixed(0) + label;
	};
	
	x.finish = function(value) {
		if(value) {
			x.set(value);
		} else {
			valuer.innerHTML = '';
		};
		container.classList.add('finished');
		
		clearInterval(timer);
	};
	
	container.classList.add('inited');
	var timer = setInterval(function(){
		
		summator = summator + ticker;
		x.set(summator);
		
		if(clicker >= time) {
			x.finish();
		};
		clicker++;
	}, 1000);
	
	return x; 
};

function blockApp() {
	document.body.classList.add('blocked');
};

function unblockApp() {
	document.body.classList.remove('blocked');
};

function errorApp() {
	document.body.classList.add('error');
};

function errorPlayApp() {
	document.body.classList.remove('error');
};

function showOK(header, desc, button, destination) {
	var headerDOM = document.getElementById('tplOk_title');
	var descDOM = document.getElementById('tplOk_desc');
	var buttonDOM = document.getElementById('tplOk_button');
	headerDOM.innerHTML = header;
	descDOM.innerHTML = desc;
	buttonDOM.innerHTML = button;
	changePage('ok', false, true);
	
	var timeoutHandler = false;
	timeoutHandler = setTimeout(function(){
		if(typeof destination === 'string') {
			changePage(destination);
		} else {
			destination();
		}
	}, 7000);
	
	if(destination) {
		buttonDOM.onclick = function(){
			clearTimeout(timeoutHandler);
			if(typeof destination === 'string') {
				changePage(destination);
			} else {
				destination();
			}
		};
	}
};

function showProblem(header, desc, button, destination) {
	var headerDOM = document.getElementById('tplProblem_title');
	var descDOM = document.getElementById('tplProblem_desc');
	var buttonDOM = document.getElementById('tplProblem_button');
	headerDOM.innerHTML = header;
	descDOM.innerHTML = desc;
	buttonDOM.innerHTML = button;
	changePage('problem', false, true);
	
	var timeoutHandler = false;
	timeoutHandler = setTimeout(function(){
		if(typeof destination === 'string') {
			changePage(destination);
		} else {
			destination();
		}
	}, 7000);
	
	if(destination) {
		buttonDOM.onclick = function(){
			clearTimeout(timeoutHandler);
			if(typeof destination === 'string') {
				changePage(destination);
			} else {
				destination();
			}
		};
	}
};

function showAttention(header, desc, button, destination) {
	var headerDOM = document.getElementById('tplAttention_title');
	var descDOM = document.getElementById('tplAttention_desc');
	var buttonDOM = document.getElementById('tplAttention_button');
	headerDOM.innerHTML = header;
	descDOM.innerHTML = desc;
	buttonDOM.innerHTML = button;
	changePage('attention', false, true);
	
	var timeoutHandler = false;
	timeoutHandler = setTimeout(function(){
		if(typeof destination === 'string') {
			changePage(destination);
		} else {
			destination();
		}
	}, 7000);
	
	if(destination) {
		buttonDOM.onclick = function(){
			clearTimeout(timeoutHandler);
			if(typeof destination === 'string') {
				changePage(destination);
			} else {
				destination();
			}
		};
	}
};

function showConfirm(header, desc, success, deny) {
	var headerDOM = document.getElementById('tplConfirm_title');
	var descDOM = document.getElementById('tplConfirm_desc');
	var buttonDOM = document.getElementById('tplConfirm_button');
	var cancelDOM = document.getElementById('tplConfirm_cancel');
	
	headerDOM.innerHTML = header;
	descDOM.innerHTML = desc;
	buttonDOM.innerHTML = 'Allow <i class="icon-ok"></i>';
	cancelDOM.innerHTML = '<i class="icon-cancel"></i> Deny';
	
	changePage('confirm', false, true);
	
	if(success) {
		buttonDOM.onclick = function(){
			if(typeof success === 'string') {
				changePage(success);
			} else {
				success(buttonDOM, cancelDOM);
			}
		};
	}
	
	if(deny) {
		cancelDOM.onclick = function(){
			if(typeof deny === 'string') {
				changePage(deny);
			} else {
				deny(buttonDOM, cancelDOM);
			}
		};
	}
};
